# FLOW 8 MIDI Controller — User Manual

A non-official cross-platform desktop MIDI controller for the [Behringer FLOW 8](https://www.behringer.com/behringer/product?modelCode=0603-AEW) mixer.

***

## Table of Contents

- [FLOW 8 MIDI Controller — User Manual](#flow-8-midi-controller--user-manual)
  - [Table of Contents](#table-of-contents)
  - [1. What is FLOW 8 MIDI Controller?](#1-what-is-flow-8-midi-controller)
  - [2. Getting Started](#2-getting-started)
    - [2.1 Requirements](#21-requirements)
    - [2.2 Connecting your FLOW 8](#22-connecting-your-flow-8)
    - [2.3 Device Selection Screen](#23-device-selection-screen)
  - [3. Navigation Bar](#3-navigation-bar)
    - [3.1 Page Tabs](#31-page-tabs)
    - [3.2 Sync Status](#32-sync-status)
    - [3.3 BLE Indicator](#33-ble-indicator)
  - [4. Mixer Page](#4-mixer-page)
    - [4.1 Input Channel Strips](#41-input-channel-strips)
    - [4.2 Bus Strips](#42-bus-strips)
  - [5. EQ Page](#5-eq-page)
    - [5.1 4-Band Channel EQ](#51-4-band-channel-eq)
    - [5.2 9-Band Bus EQ](#52-9-band-bus-eq)
  - [6. Sends Page](#6-sends-page)
  - [7. FX Page](#7-fx-page)
    - [7.1 FX Slot Controls](#71-fx-slot-controls)
    - [7.2 FX Bus Level](#72-fx-bus-level)
    - [7.3 Global FX Controls](#73-global-fx-controls)
  - [8. Snapshots Page](#8-snapshots-page)
  - [9. Settings Page](#9-settings-page)
    - [9.1 About](#91-about)
    - [9.2 Appearance \& Sync](#92-appearance--sync)
    - [9.3 Debug Log](#93-debug-log)
  - [10. Bluetooth (BLE) Sync](#10-bluetooth-ble-sync)
    - [10.1 Why Bluetooth?](#101-why-bluetooth)
    - [10.2 How It Works](#102-how-it-works)
    - [10.3 Auto-Sync](#103-auto-sync)
    - [10.4 Manual Sync](#104-manual-sync)
  - [11. FAQ — Troubleshooting](#11-faq--troubleshooting)
  - [12. For Developers](#12-for-developers)

***

## 1. What is FLOW 8 MIDI Controller?

FLOW 8 MIDI Controller is a desktop application that lets you control your Behringer FLOW 8 mixer from your computer via USB.

**What it does:**

* Full control of all mixer parameters: levels, EQ, sends, FX, mute, solo, gain, pan, compressor, low cut, phantom power, and limiter.
* FX control: preset selection, parameter editing, global FX mute, and tap tempo.
* Reads the current mixer state and reflects it in the UI (requires Bluetooth — see [Section 10](#10-bluetooth-ble-sync)).
* Loads mixer snapshots. Snapshot names are displayed when Bluetooth is connected.
* Tooltips on all sliders display real parameter values (dB, Hz, %) matching the mixer's MIDI implementation.

**What it does NOT do:**

* It does not reflect real-time physical knob/fader movements from the mixer. USB only allows sending commands to the mixer, not receiving its current state. Bluetooth partially solves this — see [Section 10](#10-bluetooth-ble-sync).
* It is not an official Behringer product.

***

## 2. Getting Started

### 2.1 Requirements

* A **Behringer FLOW 8** mixer connected via **USB** to your computer.
* **(Optional)** A Bluetooth adapter on your computer for BLE sync features (state sync, snapshots).

> **Important:** The FLOW 8 only allows **one Bluetooth connection at a time**. If the official Behringer Android/iOS app is connected to the mixer, this application will not be able to establish a Bluetooth connection (and vice-versa). Make sure to disconnect from one before connecting with the other.

### 2.2 Connecting your FLOW 8

1. Connect the FLOW 8 to your computer via USB.
2. Power on the mixer.
3. Launch the application.
4. The app automatically detects the FLOW 8 and connects to it.

### 2.3 Device Selection Screen

If the FLOW 8 is not detected, the app shows the Device Selection screen with an error message.

| Action | Description |
|--------|-------------|
| **Retry** | Scans for MIDI devices again |
| **Copy Log** | Copies the debug log to clipboard |
| **Save Log** | Saves the debug log to a file |

> **Tip:** If the FLOW 8 is not detected after confirming the USB cable is connected and restarting your computer, use **Copy Log** or **Save Log** to capture the debug log and [open an issue on GitHub](https://github.com/abelroes/flow-8-midi/issues) with the log attached.

***

## 3. Navigation Bar

The navigation bar is always visible at the top of the application once connected.

### 3.1 Page Tabs

Five main tabs: **Mixer**, **EQ**, **Sends**, **FX**, and **Snapshots**. The active tab is highlighted. A **gear icon** (⚙) on the right opens Settings.

### 3.2 Sync Status

| Indicator | Meaning |
|-----------|---------|
| 🟢 **Synced** | All UI parameters match the mixer's actual state |
| 🟡 **Unsynced** | Parameters have been changed in the UI but not yet confirmed by the mixer |

The **Sync** button (visible when BLE is connected) manually requests the current state from the mixer. The last sync timestamp is displayed next to the indicator.

### 3.3 BLE Indicator

| Color | Status |
|-------|--------|
| 🔵 Blue | Connected |
| 🟡 Yellow (blinking) | Scanning / Connecting / Authenticating |
| 🔴 Red | Error |
| ⚫ Gray | Disconnected or unavailable |

* **Double-click** the BT indicator to connect or reconnect.
* The **Disconnect** button ends the BLE session.

***

## 4. Mixer Page

The main mixing view. Displays channel strips for all 7 input channels and 3 bus outputs.

### 4.1 Input Channel Strips

Each input channel strip contains (from top to bottom):

| Control | Type | Availability |
|---------|------|--------------|
| **Channel label** | Text (e.g., "Ch 1", "USB/BT") | All channels |
| **Channel name** | Text (loaded via BLE, e.g., "Saxophone") | All channels |
| **Mute / Solo** | Toggle buttons (M and S) | All channels |
| **48V** | Toggle button (double-click to activate) | Ch 1–2 only (XLR) |
| **Gain** | Horizontal slider (-20 to +60 dB) | Ch 1–6 only |
| **Level** | Vertical slider (OFF / -70 to +10 dB) | All channels |
| **Bal** | Horizontal slider (L–C–R) | All channels |
| **Comp** | Horizontal slider (0–100%) | Ch 1–6 only |
| **Low Cut** | Horizontal slider (20–600 Hz) | Ch 1–6 only |

> **Note:** Channel 7 (USB/BT) does not have Gain, Comp, or Low Cut controls — this matches the physical mixer's behavior.

> **Note:** Phantom Power (48V) requires a **double-click** to toggle, preventing accidental activation. The first click shows a warning; the second click confirms.

### 4.2 Bus Strips

Three bus strips are displayed to the right of the input channels:

| Bus | Controls |
|-----|----------|
| **Main** | Level (vertical slider), Bal (horizontal slider), Limiter (horizontal slider) |
| **Monitor 1** | Level (vertical slider), Limiter (horizontal slider) |
| **Monitor 2** | Level (vertical slider), Limiter (horizontal slider) |

***

## 5. EQ Page

### 5.1 4-Band Channel EQ

Each input channel has a 4-band EQ with four vertical sliders: **Lo**, **LM** (Low Mid), **HM** (Hi Mid), and **Hi**.

The center position means flat (no boost/cut). Sliding up boosts the frequency, sliding down cuts it.

### 5.2 9-Band Bus EQ

The Main, Monitor 1, and Monitor 2 buses each have a 9-band graphic EQ displayed as vertical sliders:

**62 Hz · 125 Hz · 250 Hz · 500 Hz · 1 kHz · 2 kHz · 4 kHz · 8 kHz · 16 kHz**

Each bus also has a **Limiter** horizontal slider below the EQ bands.

***

## 6. Sends Page

Controls how much of each channel is sent to the monitor and FX buses.

Each input channel has four vertical sliders (faders), grouped in two sections:

| Group | Faders | Destination |
|-------|--------|-------------|
| **Monitor** | 1, 2 | Monitor 1 and Monitor 2 buses |
| **FX** | 1, 2 | FX 1 and FX 2 buses |

***

## 7. FX Page

Controls the two FX processors of the FLOW 8. The page is divided into two sections — **FX 1** (left) and **FX 2** (right) — each with its own controls and bus level fader.

### 7.1 FX Slot Controls

Each FX section contains:

| Control | Type | Description |
|---------|------|-------------|
| **Preset** | 4×4 button grid | Selects the FX algorithm. The active preset is highlighted. |
| **Param 1** | Horizontal slider | Labeled dynamically per preset (e.g., "Decay" for reverbs, "Feedback" for delays) |
| **Param 2** | Toggle button (A/B) | Labeled dynamically per preset (e.g., "Instrument/Vocal", "Dull/Bright", "Mono/Stereo") |

**FX 1 presets** are reverb-based effects (Ambience, Perc-Rev, Chamber, Room, Cathedral, Stadium, etc.), Flanger, and Chorus variants.

**FX 2 presets** are delay-based effects (Delay 1/1 through 2/1, Echo variants, Wide Echo, Ping Pong), Flanger, and Chorus variants.

### 7.2 FX Bus Level

Each FX section has a vertical **Level** fader on the side, controlling the bus return level for that FX processor.

### 7.3 Global FX Controls

At the bottom of the page, two buttons are shared between both FX slots:

| Control | Description |
|---------|-------------|
| **FX MUTE** | Mutes both FX sends globally. Turns red when active. |
| **FX 2 TAP TEMPO** | Tap repeatedly to set the FX tempo. Only affects FX 2 delay/echo presets (1–12). |

***

## 8. Snapshots Page

Displays a grid of the 15 mixer snapshot slots (4 columns). Snapshot names are loaded via Bluetooth during connection.

| Element | Description |
|---------|-------------|
| **Named slot** (highlighted) | A saved snapshot — click to load |
| **Empty slot** (dimmed) | No snapshot saved in this slot |
| **Reset to Default** | Resets the mixer to factory defaults |

> **Note:** Loading a snapshot recalls the saved preset on the mixer and triggers a resync after a short delay to update the UI. Loading works over USB — BLE is not required.

> **Note:** Snapshot names are fetched via a separate BLE request during connection. If the BLE subscribe fails (a known issue on some Windows systems), names will not appear and all slots will look the same. The snapshots themselves still work — you just won't see their names.

***

## 9. Settings Page

### 9.1 About

Displays application info: version, author, license, and a clickable link to the GitHub repository. Also includes a **Buy me a beer!** button to support the project.

### 9.2 Appearance & Sync

| Setting | Description |
|---------|-------------|
| **Theme** | Switch between light and dark themes (and all iced built-in themes) |
| **Sync Interval** | How often the app automatically syncs with the mixer via Bluetooth (Never, 30s, 1 min, 2 min, 5 min, 10 min, or 15 min) |

### 9.3 Debug Log

A scrollable log viewer showing the last 200 application events. Color-coded by severity:

| Color | Level |
|-------|-------|
| Red | Error |
| Yellow | Warning |
| Default | Info / Debug |

**Actions:**

| Button | Description |
|--------|-------------|
| **Copy to Clipboard** | Copies the full log to clipboard |
| **Save as File** | Saves to `flow8-debug-{timestamp}.log` |

***

## 10. Bluetooth (BLE) Sync

### 10.1 Why Bluetooth?

The FLOW 8's USB connection is **one-way**: the app can send commands to the mixer, but the mixer does not report its current state back over USB. This means the app has no way to know the actual position of faders, EQ settings, or any other parameter just through USB.

Bluetooth solves this. When connected via BLE, the app can ask the mixer to send a full snapshot of all its current parameters. The mixer responds over USB with a data dump that the app reads and uses to update the UI. This is the same mechanism the official Behringer mobile app uses.

**In short:**

* **USB only** — You can control the mixer, but the app starts "blind" (doesn't know the mixer's current state).
* **USB + Bluetooth** — You can control the mixer AND see its current state reflected in the UI.

### 10.2 How It Works

Once Bluetooth is connected, the app can:

1. **Read the mixer's current state** — All fader positions, EQ settings, sends, FX, mute/solo states, and channel names are fetched and reflected in the UI.
2. **Display snapshot names** — The names you assigned to your mixer snapshots are shown in the Snapshots page.

> **Note:** Loading snapshots (recalling a saved preset) works over USB without Bluetooth. BLE is only needed to display snapshot names and to read the mixer's current state.

> **Important:** The FLOW 8 only supports **one Bluetooth connection at a time**. Disconnect the official Behringer app before connecting with this application.

### 10.3 Auto-Sync

When BLE is connected and a sync interval is configured (Settings → Sync Interval), the app periodically requests the mixer's state and updates the UI to stay in sync.

### 10.4 Manual Sync

Click the **Sync** button in the navigation bar to request an immediate state update from the mixer.

***

## 11. FAQ — Troubleshooting

**Q: Snapshot names don't appear on Windows — all slots look the same.**

A: This is a known Windows limitation. The BLE subscribe required to fetch snapshot names fails on some Windows systems with the error `"The attribute cannot be written."`. Snapshots themselves work fine — you can still load them by slot number. Only the names are unavailable. There is no workaround at this time; this appears to be a platform-level BLE issue.

***

**Q: Bluetooth won't connect / BLE indicator stays red.**

A: The FLOW 8 only allows **one Bluetooth connection at a time**. The most common cause is the official Behringer mobile app (Android/iOS) still being connected. Close the official app or disable Bluetooth on your phone, then try again.

If the official app is not running and BLE still won't connect, try:

1. Disconnect and reconnect the FLOW 8 USB cable.
2. Power-cycle the FLOW 8 (turn off, wait 5 seconds, turn on).
3. Restart your computer — some Bluetooth adapters require a fresh OS-level reset to release stale connections.

***

**Q: The app says "FLOW 8 not found" even though the USB cable is plugged in.**

A: Check the following:

1. Make sure the FLOW 8 is **powered on** (the mixer must be running, not just physically plugged in).
2. Try a different USB cable or USB port — some cables are charge-only and don't carry data.
3. On Linux, make sure your user has permission to access MIDI devices (you may need to be in the `audio` group).
4. Restart your computer — some USB MIDI drivers require a fresh start after the mixer is connected for the first time.

If nothing works, use **Copy Log** or **Save Log** on the Device Selection screen and [open an issue](https://github.com/abelroes/flow-8-midi/issues) with the log attached.

***

**Q: I moved a fader on the physical mixer, but the app didn't update.**

A: This is expected behavior. USB MIDI to the FLOW 8 is **one-way** — the app can send commands to the mixer, but the mixer does not report physical control changes back over USB. To update the UI with the mixer's current state, connect via Bluetooth and use **Sync** (or enable Auto-Sync in Settings). See [Section 10](#10-bluetooth-ble-sync).

***

**Q: The app was working but now nothing responds when I move sliders.**

A: The USB connection may have dropped. This can happen if the USB cable is loose or the mixer entered a low-power state. Try:

1. Check the USB cable connection on both ends.
2. Click **Retry** on the Device Selection screen (or restart the app).
3. Power-cycle the FLOW 8.

***

**Q: Tap Tempo doesn't seem to do anything.**

A: Tap Tempo only affects **FX 2 delay/echo presets (presets 1–12)**. If FX 2 is set to a Flanger or Chorus preset (13–16), tapping will have no audible effect. Also, you need to tap **at least twice** — the mixer measures the interval between consecutive taps to calculate the BPM.

***

## 12. For Developers

* **[Developer Manual](./DEV_MANUAL.md)** — Debug tools, calibration workflow, CLI utilities, and how to build with dev-tools enabled.
* **[MIDI Implementation Research](./flow8-midi-implementation.md)** — Protocol reverse-engineering: USB MIDI, BLE authentication, SysEx dump structure and encoding.

***

*FLOW 8 MIDI Controller is free software licensed under [GNU GPLv3](../LICENSE).*
*Author: Abel Rocha Espinosa — [github.com/abelroes/flow-8-midi](https://github.com/abelroes/flow-8-midi)*

***

> 🍺 **Enjoying FLOW 8 MIDI Controller?** This project is developed and maintained in my spare time. If you find it useful, consider [buying me a beer](https://buymeacoffee.com/abelroes) — it keeps the project alive!
