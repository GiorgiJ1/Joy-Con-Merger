<div align="center">

```
 ██╗ ██████╗ ██╗   ██╗      ██████╗ ██████╗ ███╗   ██╗
 ██║██╔═══██╗╚██╗ ██╔╝     ██╔════╝██╔═══██╗████╗  ██║
 ██║██║   ██║ ╚████╔╝█████╗██║     ██║   ██║██╔██╗ ██║
 ██║██║   ██║  ╚██╔╝ ╚════╝██║     ██║   ██║██║╚██╗██║
 ██║╚██████╔╝   ██║        ╚██████╗╚██████╔╝██║ ╚████║
 ╚═╝ ╚═════╝    ╚═╝         ╚═════╝ ╚═════╝╚═╝  ╚═══╝
                    M E R G E R
```

**Merge your Nintendo Switch Joy-Cons into one virtual Xbox controller — written in Rust.**

[![Rust](https://img.shields.io/badge/Rust-1.75+-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/Windows-10%2F11-0078D4?style=flat-square&logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![ViGEmBus](https://img.shields.io/badge/ViGEmBus-required-6C3483?style=flat-square)](https://github.com/nefarius/ViGEmBus/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-22C55E?style=flat-square)](LICENSE)

<br/>

```
 Left Joy-Con  ──┐
   (Bluetooth)   ├──▶  joycon-merger  ──▶  Virtual Xbox 360 Pad  ──▶  Any PC Game
 Right Joy-Con ──┘
   (Bluetooth)
```

</div>

---

## The Problem

Windows detects a Left and Right Joy-Con as **two completely separate Bluetooth controllers**.  
Games don't know what to do with them — most expect a single unified gamepad.

**Joy-Con Merger** solves this by reading both Joy-Cons simultaneously over HID and emitting a single virtual Xbox 360 controller that every game on Windows understands natively.

---

## Features

- 🎮 **One virtual Xbox 360 pad** — appears in Device Manager just like a real controller
- ⚡ **~60 Hz polling** — low-latency HID reads, frame-paced to match display refresh
- 🔁 **Auto-reconnect** — keeps watching for Joy-Cons; reconnects automatically if they disconnect
- 🕹️ **Full analog sticks** — both Joy-Con sticks mapped with configurable dead-zone
- 🎯 **ZL / ZR as analog triggers** — full 0–255 trigger range
- 🔇 **Zero dependencies on background services** — just the ViGEmBus driver and your Bluetooth stack
- 🦀 **Pure Rust** — safe, fast, no C++ build hell

---

## Requirements

| Requirement | Details |
|---|---|
| **OS** | Windows 10 or Windows 11 |
| **ViGEmBus Driver** | [Download from GitHub](https://github.com/nefarius/ViGEmBus/releases/latest) |
| **Rust toolchain** | `x86_64-pc-windows-msvc` — install from [rustup.rs](https://rustup.rs) |
| **Visual C++ Build Tools** | Required by Rust on Windows — [download here](https://visualstudio.microsoft.com/visual-cpp-build-tools/) |
| **Bluetooth adapter** | Standard Windows Bluetooth stack |

---

## Quick Start

### 1 — Install ViGEmBus

Download and run `ViGEmBus_Setup_*.exe` from the [releases page](https://github.com/nefarius/ViGEmBus/releases/latest).  
Reboot if prompted.

### 2 — Pair your Joy-Cons via Bluetooth

1. Open **Settings → Bluetooth & devices → Add device → Bluetooth**
2. Hold the **sync button** on the Joy-Con's side rail until the lights cycle
3. Select **Joy-Con (L)** → wait for "Connected"
4. Repeat for **Joy-Con (R)**

Both should appear under **Human Interface Devices** in Device Manager.

### 3 — Build & Run

```powershell
git clone https://github.com/your-username/joycon-merger
cd joycon-merger

# Development build
cargo run

# Optimised release build
cargo build --release
.\target\release\joycon-merger.exe
```

You'll see a live status line in the terminal:

```
  Left Joy-Con : ✓ Connected   |  Right Joy-Con : ✓ Connected   |  Virtual Xbox : ✓ Active
```

Open any game, go to controller settings — it will see a standard Xbox 360 gamepad.

---

## Button Mapping

```
 LEFT JOY-CON                        RIGHT JOY-CON
 ┌─────────────────────┐             ┌─────────────────────┐
 │  ↑  ↓  ←  →  D-pad │             │    Y                │
 │  −  (Back/Select)   │             │  X   A              │
 │  L  shoulder → LB   │             │    B                │
 │  ZL trigger  → LT   │             │  +   (Start)        │
 │  Left stick  → LS   │             │  R   shoulder → RB  │
 │  L3 click    → L3   │             │  ZR  trigger  → RT  │
 └─────────────────────┘             │  Right stick  → RS  │
                                     │  R3  click    → R3  │
                                     │  Home         → Guide│
                                     └─────────────────────┘
```

| Joy-Con | Xbox 360 |
|---|---|
| D-pad (Left) | D-pad |
| − button | Back / Select |
| + button | Start |
| A B X Y (Right) | A B X Y |
| L shoulder | LB |
| ZL trigger | LT (analog) |
| R shoulder | RB |
| ZR trigger | RT (analog) |
| Left stick | Left thumbstick |
| Right stick | Right thumbstick |
| Stick click L | L3 |
| Stick click R | R3 |
| Home | Guide |

---

## Project Structure

```
joycon-merger/
├── Cargo.toml          ← dependencies & build profile
└── src/
    ├── main.rs         ← entry point, console status UI, shutdown handling
    ├── hid.rs          ← HID device scanning, open, subcommand dispatch
    ├── joycon.rs       ← input report parsing, button constants, dead-zone
    ├── merger.rs       ← 60 Hz poll loop, left+right merge logic
    └── vigem.rs        ← ViGEmBus virtual Xbox 360 controller wrapper
```

---

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                         joycon-merger                           │
│                                                                 │
│  HID scan loop          merge thread              ViGEmBus      │
│  ─────────────          ────────────              ────────      │
│  hidapi scans for  ──▶  reads L+R at ~60Hz  ──▶  XGamepad      │
│  VID=057E               parses 0x30 reports       pushed to     │
│  PID=2006 (L)           applies dead-zone         virtual       │
│  PID=2007 (R)           maps to XInput layout     Xbox 360 pad  │
└─────────────────────────────────────────────────────────────────┘
```

1. **HID scan** — `hidapi` scans all Bluetooth HID devices for Nintendo's VID (`0x057E`) and Joy-Con PIDs
2. **Report mode switch** — sends subcommand `0x03` to both Joy-Cons, enabling full `0x30` reports at ~60 Hz (sticks + all buttons + IMU)
3. **Poll loop** — non-blocking reads from both devices every ~8 ms, parses the 49-byte input report
4. **Merge** — left Joy-Con contributes left stick, D-pad, L/ZL, and −; right contributes right stick, face buttons, R/ZR, +, and Home
5. **ViGEmBus** — merged state is pushed to a virtual Xbox 360 controller that Windows and all games see as a standard XInput pad

---

## Troubleshooting

| Problem | Fix |
|---|---|
| `Could not connect to ViGEmBus` | Install/reinstall ViGEmBus, reboot |
| `Could not open Joy-Con` | Re-pair via Bluetooth; check Device Manager |
| App works but game sees no input | Run `joycon-merger.exe` as **Administrator** |
| Sticks drift or feel sluggish | Adjust `STICK_DEADZONE` in `src/joycon.rs` (default: 150) |
| Only one Joy-Con found | App waits automatically — connect the second one |

---

## Configuration

Dead-zone and center point are in `src/joycon.rs`:

```rust
pub const STICK_CENTER:   u16 = 2048;  // 12-bit midpoint
pub const STICK_DEADZONE: u16 = 150;   // increase if sticks drift
```

Poll rate is in `src/merger.rs`:

```rust
const POLL_INTERVAL: Duration = Duration::from_millis(8); // ~120 Hz
```

---

## Dependencies

| Crate | Purpose |
|---|---|
| [`hidapi`](https://crates.io/crates/hidapi) | Cross-platform HID access (Joy-Con Bluetooth) |
| [`vigem-client`](https://crates.io/crates/vigem-client) | Virtual Xbox 360 controller via ViGEmBus |
| [`crossbeam-channel`](https://crates.io/crates/crossbeam-channel) | Lock-free status channel between threads |
| [`tracing`](https://crates.io/crates/tracing) | Structured logging |
| [`anyhow`](https://crates.io/crates/anyhow) | Ergonomic error handling |

---

## License

MIT © 2024

---

<div align="center">
Built with 🦀 Rust &nbsp;·&nbsp; Made for Joy-Con owners tired of carrying an extra controller
</div>
