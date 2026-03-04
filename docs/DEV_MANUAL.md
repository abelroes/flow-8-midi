# FLOW 8 MIDI Controller — Developer Manual

This document covers the developer and debugging tools available in the application. It is intended for contributors and advanced users investigating the FLOW 8's SysEx protocol.

For general usage, see the **[User Manual](./MANUAL.md)**. For protocol details, see the **[MIDI Implementation Research](./flow8-midi-implementation.md)**.

***

## Table of Contents

- [FLOW 8 MIDI Controller — Developer Manual](#flow-8-midi-controller--developer-manual)
  - [Table of Contents](#table-of-contents)
  - [1. Overview](#1-overview)
  - [2. Enabling Dev Tools](#2-enabling-dev-tools)
    - [2.1 Debug Build](#21-debug-build)
    - [2.2 Dev-Tools Feature Flag](#22-dev-tools-feature-flag)
    - [2.3 Debug Build (Cross-compilation for Windows)](#23-debug-build-cross-compilation-for-windows)
    - [2.4 Release Build with Dev-Tools](#24-release-build-with-dev-tools)
  - [3. Dev Tools in the UI](#3-dev-tools-in-the-ui)
    - [3.1 Hex Viewer](#31-hex-viewer)
    - [3.2 Copy Hex Dump](#32-copy-hex-dump)
    - [3.3 Calibrate SysEx (UI)](#33-calibrate-sysex-ui)
    - [3.4 Run Digest (UI)](#34-run-digest-ui)
  - [4. CLI Tools](#4-cli-tools)
    - [4.1 Calibrate (CLI)](#41-calibrate-cli)
    - [4.2 Digest (CLI)](#42-digest-cli)
  - [5. Calibration Workflow](#5-calibration-workflow)
    - [5.1 What Calibration Does](#51-what-calibration-does)
    - [5.2 Full Workflow: Calibrate → Digest → Code](#52-full-workflow-calibrate--digest--code)
    - [5.3 Parameters Covered](#53-parameters-covered)
    - [5.4 Output Files](#54-output-files)
  - [6. Digest Workflow](#6-digest-workflow)
    - [6.1 What Digest Does](#61-what-digest-does)
    - [6.2 Extraction Algorithm](#62-extraction-algorithm)
    - [6.3 Output Format](#63-output-format)
  - [7. File Structure](#7-file-structure)

***

## 1. Overview

The FLOW 8's SysEx dump is a ~3068-byte binary blob containing all mixer parameters encoded as 5-byte packed IEEE 754 floats with rotating MSB positions. Mapping each parameter to its byte offsets requires an automated calibration process:

1. **Calibrate** — Systematically set each parameter to min/max via MIDI CC, capture a SysEx dump after each change.
2. **Digest** — Compare consecutive dumps to identify which bytes changed, then extract the encoding (MSB offset, data offsets, bit indices, float range).
3. **Generate code** — Produce Rust lookup tables for `sysex_parser.rs`.

Both steps are available as **UI buttons** (inside the running application) and as **standalone CLI binaries**.

***

## 2. Enabling Dev Tools

Dev tools are gated behind `#[cfg(any(debug_assertions, feature = "dev-tools"))]`. They are invisible in release builds.

### 2.1 Debug Build

```bash
cargo run
```

Debug assertions are enabled by default. All dev-tools UI elements and console output are available.

### 2.2 Dev-Tools Feature Flag

```bash
cargo run --features dev-tools
```

Enables dev-tools in a release-like build. Also required for the CLI binaries (`calibrate`, `digest`).

### 2.3 Debug Build (Cross-compilation for Windows)

When developing on WSL/Linux and testing on Windows, use a debug build to keep console output visible (release builds suppress it via `windows_subsystem = "windows"`):

```bash
cargo build --target x86_64-pc-windows-gnu
```

Then, from PowerShell on the Windows host:

```powershell
\\wsl.localhost\<distro>\home\<user>\projects\flow-8-midi\target\x86_64-pc-windows-gnu\debug\flow-8-midi.exe
```

> **Note:** The `gnu` target may require MinGW runtime DLLs. For production Windows binaries, use the `msvc` target compiled natively on Windows or via CI.

### 2.4 Release Build with Dev-Tools

```bash
cargo build --release --features dev-tools
```

The binary is output to `target/release/flow-8-midi` (or `.exe` on Windows).

***

## 3. Dev Tools in the UI

When dev-tools are enabled, additional controls appear in the **Settings** page.

### 3.1 Hex Viewer

Appears at the bottom of Settings after a SysEx dump is received. Shows the first 256 bytes of the last dump in a hex + ASCII view.

<!-- screenshot: settings_hex_viewer -->

### 3.2 Copy Hex Dump

Button in the Hex Viewer header. Copies the **entire** raw SysEx dump (all ~3068 bytes) as formatted hex text to the clipboard.

Use this to:

* Save dumps for offline analysis.
* Compare dumps in a diff tool.
* Share dump data for debugging.

### 3.3 Calibrate SysEx (UI)

**Button:** "Calibrate SysEx" in the Debug Log section.

**Requirements:**

* USB MIDI connected (for sending CC/PC messages).
* BLE connected (for triggering SysEx dumps).

**What it does:**

1. Captures a **baseline** dump.
2. For each parameter (level, gain, pan, comp, EQ, sends, mute, solo, phantom, limiter, FX presets, FX params):
   * Sends CC with value `0` (min) → triggers dump → saves.
   * Sends CC with value `127` (max) → triggers dump → saves.
3. Restores all parameters to center value (`64`).
4. Captures a **final state** dump.
5. Generates a diff report and runs the digest automatically.

**Timing:** Each step waits 800ms for the parameter to settle, then up to 5s for the dump. Total duration depends on the number of parameters (~350+ steps, ~12-15 minutes).

**Progress:** Tracked in the debug log. The button shows "Calibrating..." while running.

**Output:** Dump files saved to `calibration-data/`.

### 3.4 Run Digest (UI)

**Button:** "Run Digest" in the Debug Log section.

Runs the file-based digest on dumps stored in `calibration-data/`. This is the same algorithm as the CLI `digest` binary but reads from the shared calibration dump directory.

**Output:** `calibration-data/calibration-digest.md`.

***

## 4. CLI Tools

Standalone binaries for running calibration and digest outside the main application.

### 4.1 Calibrate (CLI)

```bash
cargo run --bin calibrate --features dev-tools
```

**Requirements:**

* FLOW 8 connected via USB (MIDI in + out).
* FLOW 8 discoverable via BLE (Bluetooth adapter required).

**What it does:**

Same as the UI calibration but runs headless in the terminal:

1. Connects to MIDI output → MIDI input → BLE (prints status for each).
2. Builds a calibration step list.
3. For each step: sends CC/PC → waits 800ms → triggers dump via BLE → waits up to 5s for SysEx response.
4. Saves each dump as a `.hex` file (hex + ASCII format).
5. Restores parameters to center.
6. Prints progress and summary.

**Output directory:** `calibration-data/`

**File naming convention:** `{channel}_{parameter}_{min|max}.hex`

Examples:

```
baseline.hex
ch1_level_min.hex
ch1_level_max.hex
ch2_gain_min.hex
main_9band_1khz_max.hex
fx1_preset_min.hex
final_state.hex
```

**Example output:**

```
=== FLOW 8 SysEx Calibration (CLI) ===

Connecting MIDI output...
  Found: "FLOW 8"
Connecting MIDI input...
  Found: "FLOW 8"
Connecting BLE...
  BLE: Scanning
  BLE: Connecting
  BLE: Authenticating
  BLE: Connected

Starting calibration: 358 steps (272 dumps, ~544s estimated)

  [  1/358] baseline                            OK (3068 bytes)
  [  2/358] ch1_level_min                       OK (3068 bytes)
  [  3/358] ch1_level_max                       OK (3068 bytes)
  ...

=== Calibration complete ===
  Dumps saved: 272/272
  Output: calibration-data/

Run `cargo run --bin digest --features dev-tools` to extract parameters.
```

### 4.2 Digest (CLI)

```bash
cargo run --bin digest --features dev-tools
```

**Requirements:**

* Calibration dumps must exist in `calibration-data/`.

**What it does:**

1. Reads all `.hex` dump files from `calibration-data/`.
2. Pairs consecutive `_min` / `_max` dumps for each parameter.
3. For each pair, computes byte diffs against the previous dump.
4. Attempts to extract the 5-byte float encoding (MSB offset + 4 data byte offsets + bit indices).
5. For boolean params (mute, solo, phantom), extracts single-byte toggle offsets.
6. Generates Rust code tables ready to paste into `sysex_parser.rs`.

**Output directory:** `calibration-data/`

**Output file:** `calibration-data/calibration-digest.md`

**Example output:**

```
Reading calibration dumps from calibration-data/...
Loaded 272/274 dumps.
Found 136 param pairs.

  ch1_level — OK (-144.0 .. 10.0)
  ch2_level — OK (-144.0 .. 10.0)
  ...
  ch1_mute — OK (bool, offset=0x0092)
  ...
  main_9band_1khz — FAILED

Done: 120/136 params extracted. Digest saved to calibration-data/calibration-digest.md
```

***

## 5. Calibration Workflow

### 5.1 What Calibration Does

The calibration process maps MIDI CC parameters to their byte offsets inside the SysEx dump. For each parameter:

```
1. Send CC min (value=0) via USB MIDI
2. Wait 800ms for mixer to settle
3. Trigger SysEx dump via BLE (command 0x4B)
4. Receive dump via USB MIDI input
5. Save dump as {param}_min.hex

6. Send CC max (value=127) via USB MIDI
7. Wait 800ms
8. Trigger dump → receive → save as {param}_max.hex
```

By comparing the min and max dumps, the digest can identify exactly which bytes encode that parameter.

### 5.2 Full Workflow: Calibrate → Digest → Code

```
┌─────────────────────────────────────────────────────┐
│ Step 1: Calibrate                                   │
│                                                     │
│   cargo run --bin calibrate --features dev-tools     │
│   (or click "Calibrate SysEx" in Settings)          │
│                                                     │
│   Output: calibration-data/*.hex              │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│ Step 2: Digest                                      │
│                                                     │
│   cargo run --bin digest --features dev-tools        │
│   (or click "Run Digest" in Settings)               │
│                                                     │
│   Output: calibration-data/calibration-digest.md    │
│           (includes generated Rust code)            │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│ Step 3: Integrate                                   │
│                                                     │
│   Copy FloatParam / BoolParam tables from the       │
│   digest into src/service/sysex_parser.rs           │
└─────────────────────────────────────────────────────┘
```

### 5.3 Parameters Covered

**Input Channels** (Ch 1–7, 7 channels × 15 params = 210 dumps):

| Parameter | CC# | Type |
|-----------|-----|------|
| Level | 7 | Float |
| Gain | 8 | Float |
| Pan | 10 | Float |
| Compressor | 11 | Float |
| Low Cut | 9 | Float |
| EQ Low | 1 | Float |
| EQ Low Mid | 2 | Float |
| EQ Hi Mid | 3 | Float |
| EQ Hi | 4 | Float |
| Send Mon 1 | 14 | Float |
| Send Mon 2 | 15 | Float |
| Send FX 1 | 16 | Float |
| Send FX 2 | 17 | Float |
| Mute | 5 | Bool |
| Solo | 6 | Bool |

**Phantom Power** (Ch 1–2 only, CC# 12, Bool — 4 dumps)

**Buses** (Main, Mon1, Mon2, FX1, FX2 — 5 buses × 12 params = 120 dumps):

| Parameter | CC# | Type |
|-----------|-----|------|
| Level | 7 | Float |
| Limiter | 8 | Float |
| Balance | 10 | Float |
| 9-Band EQ (62Hz–16kHz) | 11–19 | Float |

**FX Control** (FX1, FX2 — 2 slots × 3 params = 12 dumps):

| Parameter | Message | Type |
|-----------|---------|------|
| Param 1 | CC 1 | Float |
| Param 2 | CC 2 | Float |
| Preset | Program Change 0–15 | Float |

**Total:** ~350 steps, ~272 dumps (+ baseline + final state + restore steps).

### 5.4 Output Files

| Source | Output Directory | Files |
|--------|-----------------|-------|
| **Calibrate (CLI / UI)** | `calibration-data/` | `*.hex` dump files + `calibration-report.md` |
| **Digest (CLI / UI)** | `calibration-data/` | `calibration-digest.md` (includes generated Rust code) |

***

## 6. Digest Workflow

### 6.1 What Digest Does

The digest takes calibration dump pairs (min + max for each parameter) and reverse-engineers the SysEx byte encoding:

1. **Diff computation** — Compares each `_min` dump against the previous dump, and `_max` against `_min`, to find changed byte offsets.
2. **Float parameter extraction** — For each set of changed offsets, tries all possible combinations of 1 MSB byte + 4 data bytes within a search radius, restoring bit 7 from the MSB and interpreting the result as an IEEE 754 little-endian float.
3. **Boolean parameter extraction** — For mute/solo/phantom, looks for single bytes that flip between `0x00` and `0x01`.
4. **Validation** — Checks that extracted float values fall within expected ranges (e.g., Level: -144 to +10 dB, Pan: -1.5 to +1.5, Gain: -20 to +60 dB).
5. **Code generation** — Produces Rust `const` arrays of `FloatParam` and `BoolParam` structs.

### 6.2 Extraction Algorithm

For float parameters, the algorithm searches around the changed offsets:

```
For each candidate MSB offset in [changed_min - 8 .. changed_max + 8]:
  For each combination of 4 data byte offsets within ±7 of MSB:
    For each bit assignment (unique bits 0–6 for each data byte):
      Restore bit 7 of each data byte from MSB
      Interpret 4 bytes as little-endian f32
      If both min and max floats are valid and within expected range:
        Score by overlap with actual changed offsets
        Keep best match
```

### 6.3 Output Format

The digest generates a Markdown file with:

1. **Per-parameter section** — Shows diff offsets, extraction result (OK/FAILED), MSB offset, data offsets, bit indices, and float range.
2. **Generated Rust code** — Ready-to-paste `const` arrays grouped by parameter type:

```rust
const CHANNEL_LEVELS: [FloatParam; 7] = [
    FloatParam { msb_off: 0x0067, data_offs: [0x0068, 0x0069, 0x006A, 0x006B], bit_indices: [0, 1, 2, 3] }, // ch1 (-144.0..10.0)
    // ...
];
```

3. **Failed extractions** — List of parameters that could not be auto-extracted (may need manual analysis using the [Implementation Research](./flow8-midi-implementation.md) document).

***

## 7. File Structure

```
flow-8-midi/
├── src/
│   ├── main.rs                       # Application entry point and update loop
│   ├── logger.rs                     # Custom log macros (log!, log_debug!)
│   ├── bin/
│   │   ├── calibrate.rs              # CLI calibration binary
│   │   ├── digest.rs                 # CLI digest binary
│   │   └── analyze_names.rs          # Channel name encoding validator
│   ├── model/
│   │   ├── mod.rs
│   │   ├── flow8.rs                  # Main application state (FLOW8Controller)
│   │   ├── channels.rs              # Channel, bus, FX data structures & FX preset definitions
│   │   ├── message.rs               # UI-driven messages (InterfaceMessage enum)
│   │   └── page.rs                  # Page navigation enum
│   ├── service/
│   │   ├── mod.rs
│   │   ├── ble.rs                   # BLE connection, auth, dump trigger, snapshot fetch
│   │   ├── midi.rs                  # Raw MIDI send (CC, PC, NoteOn)
│   │   ├── midi_mapper.rs           # Maps InterfaceMessage → MIDI commands
│   │   ├── sysex_parser.rs          # SysEx dump parser (uses extracted offset tables)
│   │   └── sysex_calibration.rs     # UI calibration state machine + file-based digest
│   └── view/
│       ├── mod.rs
│       ├── widgets.rs               # Reusable widgets (sliders, tooltips) + value formatters
│       ├── device_select_page.rs    # Device selection / error screen
│       ├── nav_bar.rs               # Navigation bar (tabs, sync status, BLE indicator)
│       ├── mixer_page.rs            # Mixer page (channel strips, bus strips)
│       ├── eq_page.rs               # EQ page (4-band channel EQ, 9-band bus EQ)
│       ├── sends_page.rs            # Sends page (monitor + FX send levels)
│       ├── fx_page.rs               # FX page (FX1/FX2 presets, params, mute, tap tempo)
│       ├── snapshots_page.rs        # Snapshots page (load/reset)
│       └── settings_page.rs         # Settings page (about, theme, sync interval, debug log)
├── docs/
│   ├── MANUAL.md                    # User manual
│   ├── DEV_MANUAL.md                # This document
│   └── flow8-midi-implementation.md # Protocol research (MIDI, BLE, SysEx)
├── resources/                       # Icons, screenshots
└── calibration-data/                # Calibration dumps & digest output (generated, gitignored)
```

***

*See also: [User Manual](./MANUAL.md) · [MIDI Implementation Research](./flow8-midi-implementation.md)*

***

> 🍺 **Enjoying FLOW 8 MIDI Controller?** This project is developed and maintained in my spare time. If you find it useful, consider [buying me a beer](https://buymeacoffee.com/abelroes) — it keeps the project alive!
