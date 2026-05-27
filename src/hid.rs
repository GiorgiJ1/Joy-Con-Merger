// joycon-merger/src/hid.rs
// HID scanning and Joy-Con device management.
//
// On Windows 11, Bluetooth HID output reports are blocked by the OS for
// security reasons — we cannot send subcommands to switch report mode.
// Instead we open the device read-only and accept whatever report mode
// the Joy-Con is already in (usually 0x3F simple input or 0x30 full input).

use anyhow::{bail, Result};
use hidapi::{HidApi, HidDevice};
use tracing::{info, warn};

use crate::joycon::{JOYCON_L_PID, JOYCON_R_PID, NINTENDO_VID, Side};

pub struct JoyConDevice {
    pub device: HidDevice,
    pub side:   Side,
}

impl JoyConDevice {
    /// Open a Joy-Con by iterating all matching HID interface paths.
    /// Does NOT send any output reports — works on Windows 11 without admin rights.
    pub fn open(api: &HidApi, side: Side) -> Result<Self> {
        let pid = match side {
            Side::Left  => JOYCON_L_PID,
            Side::Right => JOYCON_R_PID,
        };

        let paths: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == NINTENDO_VID && d.product_id() == pid)
            .collect();

        if paths.is_empty() {
            bail!(
                "{:?} Joy-Con not found (VID={NINTENDO_VID:#06x} PID={pid:#06x})",
                side
            );
        }

        // Try each HID interface path — pick the first one that opens and
        // actually produces input data (usage page 0x0001 = Generic Desktop,
        // usage 0x0005 = Gamepad is what we want)
        let mut best: Option<HidDevice> = None;

        for info in &paths {
            // Prefer the gamepad usage interface if the OS exposes usage info
            let is_gamepad = info.usage_page() == 0x0001 && info.usage() == 0x0005;

            match api.open_path(info.path()) {
                Ok(dev) => {
                    let _ = dev.set_blocking_mode(false);
                    if is_gamepad || best.is_none() {
                        best = Some(dev);
                        if is_gamepad {
                            break; // Perfect match — stop looking
                        }
                    }
                }
                Err(e) => {
                    warn!("{:?} Joy-Con: could not open path {:?}: {e}", side, info.path());
                }
            }
        }

        match best {
            Some(device) => {
                info!("{:?} Joy-Con opened (read-only, no subcommands).", side);
                Ok(JoyConDevice { device, side })
            }
            None => bail!("{:?} Joy-Con: failed to open any HID interface", side),
        }
    }

    /// Non-blocking read — returns None if no data available yet.
    pub fn read_report(&self, buf: &mut [u8]) -> Result<Option<usize>> {
        match self.device.read(buf) {
            Ok(0)  => Ok(None),
            Ok(n)  => Ok(Some(n)),
            Err(e) => {
                warn!("{:?} Joy-Con read error: {e}", self.side);
                Err(e.into())
            }
        }
    }
}

/// Scan the HID bus — returns (left_found, right_found).
pub fn scan_for_joycons(api: &HidApi) -> (bool, bool) {
    let mut left  = false;
    let mut right = false;
    for dev in api.device_list() {
        if dev.vendor_id() == NINTENDO_VID {
            match dev.product_id() {
                JOYCON_L_PID => left  = true,
                JOYCON_R_PID => right = true,
                _            => {}
            }
        }
    }
    (left, right)
}