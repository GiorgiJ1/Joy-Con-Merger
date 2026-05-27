// joycon-merger/src/main.rs
// Entry point: launches the egui desktop GUI and merger backend thread.

mod hid;
mod joycon;
mod merger;
mod vigem;
mod ui;

use anyhow::Result;
use crossbeam_channel::unbounded;
use std::sync::{Arc, atomic::AtomicBool};
use tracing::error;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let running = Arc::new(AtomicBool::new(true));
    let (status_tx, status_rx) = unbounded::<merger::StatusEvent>();

    // Spawn merger backend thread
    let running_merger = running.clone();
    std::thread::spawn(move || {
        if let Err(e) = merger::run(running_merger, status_tx) {
            error!("Merger thread error: {e:#}");
        }
    });

    // Run egui frontend (blocks until window closed)
    ui::run(running, status_rx)?;

    Ok(())
}