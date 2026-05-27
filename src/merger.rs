// joycon-merger/src/merger.rs
// Core loop: reads both Joy-Cons, merges input, drives virtual Xbox controller.
//
// Since we cannot send subcommands on Windows 11, we accept whatever report
// mode the Joy-Con is already sending — 0x30 (full) or 0x3F (simple).

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam_channel::Sender;
use hidapi::HidApi;
use tracing::{info, warn};

use crate::hid::{JoyConDevice, scan_for_joycons};
use crate::joycon::{JoyConInput, Side, apply_deadzone, STICK_CENTER, STICK_DEADZONE};
use crate::vigem::{VirtualXbox, build_gamepad};

#[derive(Debug)]
pub enum StatusEvent {
    LeftConnected,
    RightConnected,
    LeftDisconnected,
    RightDisconnected,
    VirtualReady,
    VirtualDropped,
    InputError(String),
}

const POLL_INTERVAL: Duration = Duration::from_millis(8);

pub fn run(running: Arc<AtomicBool>, status: Sender<StatusEvent>) -> Result<()> {
    info!("Merger thread started.");

    'outer: while running.load(Ordering::SeqCst) {
        // ── Wait until both Joy-Cons are visible on the HID bus ───────────
        loop {
            if !running.load(Ordering::SeqCst) { break 'outer; }
            if let Ok(api) = HidApi::new() {
                let (has_l, has_r) = scan_for_joycons(&api);
                if has_l && has_r { break; }
                if has_l && !has_r {
                    info!("Left Joy-Con found — waiting for Right...");
                }
                if has_r && !has_l {
                    info!("Right Joy-Con found — waiting for Left...");
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        let api = HidApi::new()?;

        // ── Open Left Joy-Con ─────────────────────────────────────────────
        let left = match JoyConDevice::open(&api, Side::Left) {
            Ok(d)  => { status.send(StatusEvent::LeftConnected).ok(); d }
            Err(e) => {
                warn!("Left Joy-Con open failed: {e}");
                std::thread::sleep(Duration::from_secs(2));
                continue 'outer;
            }
        };

        // ── Open Right Joy-Con ────────────────────────────────────────────
        let right = match JoyConDevice::open(&api, Side::Right) {
            Ok(d)  => { status.send(StatusEvent::RightConnected).ok(); d }
            Err(e) => {
                warn!("Right Joy-Con open failed: {e}");
                std::thread::sleep(Duration::from_secs(2));
                continue 'outer;
            }
        };

        // ── Create virtual Xbox controller ────────────────────────────────
        let mut xbox = match VirtualXbox::new() {
            Ok(v)  => { status.send(StatusEvent::VirtualReady).ok(); v }
            Err(e) => {
                warn!("ViGEmBus error: {e}");
                status.send(StatusEvent::InputError(e.to_string())).ok();
                std::thread::sleep(Duration::from_secs(5));
                continue 'outer;
            }
        };

        info!("Both Joy-Cons connected — merging input.");

        let mut left_state  = JoyConInput::default();
        let mut right_state = JoyConInput::default();
        let mut buf = [0u8; 64];
        let mut report_type_logged = false;

        // ── Main poll loop ────────────────────────────────────────────────
        loop {
            if !running.load(Ordering::SeqCst) { break 'outer; }
            let frame_start = Instant::now();

            // Read Left Joy-Con — try full report first, fall back to simple
            match left.read_report(&mut buf) {
                Ok(Some(n)) => {
                    if !report_type_logged {
                        info!("Receiving report type {:#04x} (len={n})", buf[0]);
                        report_type_logged = true;
                    }
                    if let Some(inp) = JoyConInput::parse_full_report(&buf, Side::Left) {
                        left_state = inp;
                    } else if let Some(inp) = JoyConInput::parse_simple_report(&buf, Side::Left) {
                        left_state = inp;
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Left Joy-Con disconnected: {e}");
                    status.send(StatusEvent::LeftDisconnected).ok();
                    break;
                }
            }

            // Read Right Joy-Con
            match right.read_report(&mut buf) {
                Ok(Some(_)) => {
                    if let Some(inp) = JoyConInput::parse_full_report(&buf, Side::Right) {
                        right_state = inp;
                    } else if let Some(inp) = JoyConInput::parse_simple_report(&buf, Side::Right) {
                        right_state = inp;
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Right Joy-Con disconnected: {e}");
                    status.send(StatusEvent::RightDisconnected).ok();
                    break;
                }
            }

            // Push merged state to virtual Xbox controller
            let gamepad = merge(&left_state, &right_state);
            if let Err(e) = xbox.update(&gamepad) {
                warn!("ViGEmBus update failed: {e}");
                status.send(StatusEvent::VirtualDropped).ok();
                break;
            }

            let elapsed = frame_start.elapsed();
            if elapsed < POLL_INTERVAL {
                std::thread::sleep(POLL_INTERVAL - elapsed);
            }
        }

        status.send(StatusEvent::VirtualDropped).ok();
        info!("Connection lost — will retry...");
        std::thread::sleep(Duration::from_secs(1));
    }

    info!("Merger thread exiting.");
    Ok(())
}

fn merge(l: &JoyConInput, r: &JoyConInput) -> vigem_client::XGamepad {
    let lx = apply_deadzone(l.stick_x, STICK_CENTER, STICK_DEADZONE);
    let ly = apply_deadzone(l.stick_y, STICK_CENTER, STICK_DEADZONE);
    let rx = apply_deadzone(r.stick_x, STICK_CENTER, STICK_DEADZONE);
    let ry = apply_deadzone(r.stick_y, STICK_CENTER, STICK_DEADZONE);

    let lt: u8 = if l.btn_zl { 255 } else { 0 };
    let rt: u8 = if r.btn_zr { 255 } else { 0 };

    build_gamepad(
        r.btn_a, r.btn_b, r.btn_x, r.btn_y,
        r.btn_plus, l.btn_minus,
        l.btn_l, r.btn_r, lt, rt,
        l.btn_lstick, r.btn_rstick,
        l.btn_up, l.btn_down, l.btn_left, l.btn_right,
        r.btn_home, false,
        lx, ly, rx, ry,
    )
}