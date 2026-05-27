// joycon-merger/src/vigem.rs
// Wraps vigem-client to expose a simple "update gamepad" interface.
//
// ViGEmBus driver must be installed on the host machine.
// Download: https://github.com/nefarius/ViGEmBus/releases

use anyhow::{Context, Result};
use vigem_client::{Client, Xbox360Wired, XButtons, XGamepad, TargetId};
use tracing::info;

pub struct VirtualXbox {
    target: Xbox360Wired<Client>,
}

impl VirtualXbox {
    /// Connect to ViGEmBus and plug in a virtual Xbox 360 controller.
    pub fn new() -> Result<Self> {
        let client = Client::connect()
            .context("Could not connect to ViGEmBus. Is the driver installed? See README.")?;

        let id = TargetId::XBOX360_WIRED;
        let mut target = Xbox360Wired::new(client.try_clone()?, id);
        target.plugin().context("Failed to plug in virtual Xbox 360 controller")?;
        target.wait_ready().context("Virtual controller never became ready")?;

        info!("Virtual Xbox 360 controller is live.");
        Ok(VirtualXbox { target })
    }

    /// Push a new gamepad state to Windows.
    pub fn update(&mut self, state: &XGamepad) -> Result<()> {
        self.target.update(state)?;
        Ok(())
    }
}

/// Build an XGamepad from our merged button/stick state.
/// XButtons::UP etc. are plain u16 constants — OR them into a raw u16,
/// then construct XButtons { raw }.
pub fn build_gamepad(
    a: bool, b: bool, x: bool, y: bool,
    start: bool, back: bool,
    lb: bool, rb: bool, lt_val: u8, rt_val: u8,
    ls: bool, rs: bool,
    dpad_up: bool, dpad_down: bool, dpad_left: bool, dpad_right: bool,
    _home: bool, guide: bool,
    lx: i16, ly: i16, rx: i16, ry: i16,
) -> XGamepad {
    let mut raw: u16 = 0;
    if dpad_up    { raw |= XButtons::UP; }
    if dpad_down  { raw |= XButtons::DOWN; }
    if dpad_left  { raw |= XButtons::LEFT; }
    if dpad_right { raw |= XButtons::RIGHT; }
    if start      { raw |= XButtons::START; }
    if back       { raw |= XButtons::BACK; }
    if ls         { raw |= XButtons::LTHUMB; }
    if rs         { raw |= XButtons::RTHUMB; }
    if lb         { raw |= XButtons::LB; }
    if rb         { raw |= XButtons::RB; }
    if a          { raw |= XButtons::A; }
    if b          { raw |= XButtons::B; }
    if x          { raw |= XButtons::X; }
    if y          { raw |= XButtons::Y; }
    if guide      { raw |= XButtons::GUIDE; }

    XGamepad {
        buttons:       XButtons { raw },
        left_trigger:  lt_val,
        right_trigger: rt_val,
        thumb_lx: lx,
        thumb_ly: ly,
        thumb_rx: rx,
        thumb_ry: ry,
    }
}