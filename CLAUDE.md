# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Desktop app for real-time cycling dynamics visualization from a Favero Assioma power meter via ANT+ USB stick. Built with Tauri v2 (Rust backend + vanilla JS/HTML/CSS frontend). Currently supports live ANT+ data; .fit file import is planned but not yet implemented.

## Build & Development Commands

```bash
cargo tauri dev          # Run development server (hot-reload frontend, rebuilds Rust on change)
cargo tauri build        # Production build with bundling
```

Rust-only commands (from `src-tauri/`):
```bash
cargo build              # Build Rust backend only
cargo check              # Type-check without building
cargo clippy             # Lint Rust code
cargo test               # Run tests (none exist yet)
```

## Architecture

```
src/                     # Frontend (Tauri WebView)
  index.html             # UI layout - power, cadence, L/R dynamics cards
  main.js                # Tauri event listener + Canvas 2D phase visualization
  styles.css             # Dark theme (#1a1a1a bg, #00d2ff accent)

src-tauri/src/           # Backend (Rust)
  main.rs                # Binary entry point
  lib.rs                 # Tauri setup, registers `connect_ant` command
  ant_driver.rs          # USB device detection, ANT+ channel init, bulk read loop
  protocol.rs            # ANT+ broadcast message parsing → CyclingData struct
```

### Data Flow

1. **ant_driver.rs** finds ANT+ USB stick (Garmin VID `0x0fcf` or Silicon Labs `0x10c4`), opens device, configures ANT+ channel
2. Spawns a thread that reads 64-byte USB bulk transfers in a loop
3. Parses ANT+ message framing: `[SYNC(0xA4), LEN, MSG_ID, DATA..., CHECKSUM]`
4. Broadcast data (MSG_ID `0x4E`) is passed to **protocol.rs** page parsers:
   - Page `0x10`: instant power (watts) + cadence (RPM)
   - Page `0x13`: torque effectiveness + pedal smoothness (L/R)
   - Page `0x19`: power phase angles (L/R start/end degrees)
5. Parsed data emits `cycling-data` Tauri event with serialized `CyclingData`
6. **main.js** listens for the event, updates DOM elements and redraws canvas arcs

### Key Type

`CyclingData` (protocol.rs) is the central struct passed via Tauri IPC — contains instant_power, cadence, L/R torque effectiveness, L/R pedal smoothness, L/R phase angles, and platform center offset (PCO).

## ANT+ Protocol Notes

- Network key: standard ANT+ public key
- Radio frequency: 57 (2457 MHz), channel period: 8182 (~4Hz updates)
- Device type 11 = bike power sensor
- Checksum: XOR of all message bytes must equal 0
- Power phase angles encoded as 0-255 mapped to 0-360 degrees

## Platform Considerations

- Developed on macOS; uses `detach_kernel_driver()` to claim USB from OS
- Requires libusb (via rusb crate) — users need USB access permissions
- CSP is disabled in tauri.conf.json (development convenience)
