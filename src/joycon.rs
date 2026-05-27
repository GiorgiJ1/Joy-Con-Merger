// joycon-merger/src/joycon.rs
// Joy-Con HID protocol: vendor/product IDs, input report parsing, button layout.
//
// Reference: https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering

// ── USB/BT Vendor & Product IDs ─────────────────────────────────────────────
pub const NINTENDO_VID: u16 = 0x057E;
pub const JOYCON_L_PID: u16 = 0x2006;
pub const JOYCON_R_PID: u16 = 0x2007;

// ── Joy-Con subcommand IDs ───────────────────────────────────────────────────
pub const SUBCMD_SET_INPUT_REPORT_MODE: u8 = 0x03;
pub const SUBCMD_ENABLE_VIBRATION:      u8 = 0x48;
pub const SUBCMD_ENABLE_IMU:            u8 = 0x40;

// Input report mode: 0x30 = full input (sticks + buttons + IMU @ 60 Hz)
pub const INPUT_REPORT_FULL: u8 = 0x30;
// Input report mode: 0x3F = simple button/stick (lower bandwidth)
pub const INPUT_REPORT_SIMPLE: u8 = 0x3F;

// ── Side marker ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Left, Right }

// ── Raw input from a single Joy-Con (standard full report 0x30) ─────────────
#[derive(Debug, Clone, Default)]
pub struct JoyConInput {
    pub side: Option<Side>,

    // Buttons (Left Joy-Con)
    pub btn_left:   bool,  // ← D-pad
    pub btn_right:  bool,  // → D-pad
    pub btn_up:     bool,  // ↑ D-pad
    pub btn_down:   bool,  // ↓ D-pad
    pub btn_sl_l:   bool,
    pub btn_sr_l:   bool,
    pub btn_minus:  bool,
    pub btn_lstick: bool,  // L3

    // Buttons (Right Joy-Con)
    pub btn_a:      bool,
    pub btn_b:      bool,
    pub btn_x:      bool,
    pub btn_y:      bool,
    pub btn_sl_r:   bool,
    pub btn_sr_r:   bool,
    pub btn_plus:   bool,
    pub btn_rstick: bool,  // R3
    pub btn_home:   bool,
    pub btn_capture:bool,

    // Shoulder / trigger (both sides share L/ZL or R/ZR)
    pub btn_l:  bool,
    pub btn_zl: bool,
    pub btn_r:  bool,
    pub btn_zr: bool,

    // Analog sticks — raw 12-bit values (0..=4095)
    pub stick_x: u16,
    pub stick_y: u16,
}

impl JoyConInput {
    /// Parse a 0x30 full-input HID report (49 bytes).
    /// `buf` starts at the first byte of the HID report (the report ID byte).
    pub fn parse_full_report(buf: &[u8], side: Side) -> Option<Self> {
        if buf.len() < 13 {
            return None;
        }

        // Report ID must be 0x30 for full input mode
        if buf[0] != 0x30 {
            return None;
        }

        // Bytes 3-5: button bitfields
        // Byte 3: Right Joy-Con buttons
        // Byte 4: Shared buttons (Minus/Plus, Stick clicks, Home, Capture)
        // Byte 5: Left Joy-Con buttons
        let btn_right_byte  = buf[3];
        let btn_shared_byte = buf[4];
        let btn_left_byte   = buf[5];

        // Bytes 6-8: Left stick  (3 bytes → two 12-bit values)
        // Bytes 9-11: Right stick
        let (lx, ly) = decode_stick(&buf[6..9]);
        let (rx, ry) = decode_stick(&buf[9..12]);

        let (stick_x, stick_y) = match side {
            Side::Left  => (lx, ly),
            Side::Right => (rx, ry),
        };

        Some(JoyConInput {
            side: Some(side),

            // ── Left Joy-Con buttons (byte 5) ────────────────────────────
            btn_down:   btn_left_byte & 0x01 != 0,
            btn_up:     btn_left_byte & 0x02 != 0,
            btn_right:  btn_left_byte & 0x04 != 0,
            btn_left:   btn_left_byte & 0x08 != 0,
            btn_sl_l:   btn_left_byte & 0x10 != 0,
            btn_sr_l:   btn_left_byte & 0x20 != 0,
            btn_l:      btn_left_byte & 0x40 != 0,
            btn_zl:     btn_left_byte & 0x80 != 0,

            // ── Right Joy-Con buttons (byte 3) ───────────────────────────
            btn_y:      btn_right_byte & 0x01 != 0,
            btn_x:      btn_right_byte & 0x02 != 0,
            btn_b:      btn_right_byte & 0x04 != 0,
            btn_a:      btn_right_byte & 0x08 != 0,
            btn_sl_r:   btn_right_byte & 0x10 != 0,
            btn_sr_r:   btn_right_byte & 0x20 != 0,
            btn_r:      btn_right_byte & 0x40 != 0,
            btn_zr:     btn_right_byte & 0x80 != 0,

            // ── Shared buttons (byte 4) ──────────────────────────────────
            btn_minus:   btn_shared_byte & 0x01 != 0,
            btn_plus:    btn_shared_byte & 0x02 != 0,
            btn_rstick:  btn_shared_byte & 0x04 != 0,
            btn_lstick:  btn_shared_byte & 0x08 != 0,
            btn_home:    btn_shared_byte & 0x10 != 0,
            btn_capture: btn_shared_byte & 0x20 != 0,

            stick_x,
            stick_y,
        })
    }

    /// Parse a simpler 0x3F report (used before we switch mode — 12 bytes).
    pub fn parse_simple_report(buf: &[u8], side: Side) -> Option<Self> {
        if buf.len() < 12 || buf[0] != 0x3F {
            return None;
        }
        let btn1 = buf[1];
        let btn2 = buf[2];
        // Sticks are 4-bit each in simple report
        let sx = (buf[4] as u16) | (((buf[5] & 0x0F) as u16) << 8);
        let sy = ((buf[5] >> 4) as u16) | ((buf[6] as u16) << 4);

        let (stick_x, stick_y) = match side {
            Side::Left  => (sx, sy),
            Side::Right => {
                let rx = (buf[7] as u16) | (((buf[8] & 0x0F) as u16) << 8);
                let ry = ((buf[8] >> 4) as u16) | ((buf[9] as u16) << 4);
                (rx, ry)
            }
        };

        Some(JoyConInput {
            side: Some(side),
            btn_y:      btn1 & 0x01 != 0,
            btn_x:      btn1 & 0x02 != 0,
            btn_b:      btn1 & 0x04 != 0,
            btn_a:      btn1 & 0x08 != 0,
            btn_r:      btn1 & 0x40 != 0,
            btn_zr:     btn1 & 0x80 != 0,
            btn_minus:  btn2 & 0x01 != 0,
            btn_plus:   btn2 & 0x02 != 0,
            btn_rstick: btn2 & 0x04 != 0,
            btn_lstick: btn2 & 0x08 != 0,
            btn_home:   btn2 & 0x10 != 0,
            btn_down:   btn2 & 0x01 != 0,
            btn_up:     btn2 & 0x02 != 0,
            btn_left:   btn2 & 0x04 != 0,
            btn_right:  btn2 & 0x08 != 0,
            btn_l:      btn2 & 0x40 != 0,
            btn_zl:     btn2 & 0x80 != 0,
            stick_x,
            stick_y,
            ..Default::default()
        })
    }
}

/// Decode two 12-bit stick axes from 3 packed bytes.
/// Layout: [byte0=LSB_x(8), byte1=MSB_x(4)|LSB_y(4), byte2=MSB_y(8)]
fn decode_stick(bytes: &[u8]) -> (u16, u16) {
    let x = (bytes[0] as u16) | (((bytes[1] & 0x0F) as u16) << 8);
    let y = ((bytes[1] >> 4) as u16) | ((bytes[2] as u16) << 4);
    (x, y)
}

/// Apply a simple circular dead-zone to a raw 12-bit axis value.
/// Returns a value in -32768..=32767 (i16 range for XInput).
pub fn apply_deadzone(raw: u16, center: u16, deadzone: u16) -> i16 {
    let signed = raw as i32 - center as i32;
    if signed.unsigned_abs() as u16 <= deadzone {
        return 0;
    }
    // Scale to i16 range
    let max = 2048i32;
    let scaled = (signed * 32767) / max;
    scaled.clamp(-32768, 32767) as i16
}

pub const STICK_CENTER:   u16 = 2048;
pub const STICK_DEADZONE: u16 = 150;