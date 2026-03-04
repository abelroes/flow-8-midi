# Behringer FLOW 8 — MIDI Implementation Research

> This is a technical reference for the FLOW 8's MIDI and BLE protocol internals. For general usage instructions, see the **[User Manual](./MANUAL.md)**. For calibration and digest tool usage, see the **[Developer Manual](./DEV_MANUAL.md)**.

## Overview

The Behringer FLOW 8 is a compact digital mixer that supports remote control via:

* **USB MIDI** (Control Change / Program Change — one-way, host→mixer only)
* **BLE (Bluetooth Low Energy)** — proprietary protocol reverse-engineered and implemented in our desktop app

This document consolidates all findings from reverse-engineering the FLOW 8's MIDI and BLE communication.

***

## 1. USB MIDI — Control Change / Program Change

The FLOW 8 accepts standard MIDI CC and PC messages via USB. Communication is **one-way**: the mixer receives commands but does **not** send CC feedback when physical controls are adjusted.

### 1.1 MIDI Channel Assignment (0-indexed)

| MIDI Channel (0-idx) | MIDI Channel (1-idx) | Target |
|---|---|---|
| 0–6 | 1–7 | Input Channels 1–7 (Ch 7 = USB/BT) |
| 7 | 8 | Main Bus |
| 8 | 9 | Monitor 1 Bus |
| 9 | 10 | Monitor 2 Bus |
| 10 | 11 | FX 1 Bus |
| 11 | 12 | FX 2 Bus |
| 12 | 13 | *(not used)* |
| 13 | 14 | FX 1 Slot Control |
| 14 | 15 | FX 2 Slot Control |
| 15 | 16 | Global (Snapshots, FX Mute, Tap Tempo) |

### 1.2 Input Channels (Ch 0–6) — Control Change

| CC# | Parameter | CC Range | Value Range | Notes |
|---|---|---|---|---|
| 1 | EQ Low | 0–127 | -15 to +15 dB | 64 = 0.0 dB (center) |
| 2 | EQ Low Mid | 0–127 | -15 to +15 dB | 64 = 0.0 dB (center) |
| 3 | EQ Hi Mid | 0–127 | -15 to +15 dB | 64 = 0.0 dB (center) |
| 4 | EQ High | 0–127 | -15 to +15 dB | 64 = 0.0 dB (center) |
| 5 | Mute | 0 / 1-127 | OFF / MUTE | Switch: 0 = mute off, 1-127 = mute on |
| 6 | Solo | 0 / 1-127 | OFF / SOLO | Switch: 0 = solo off, 1-127 = solo on |
| 7 | Level (Fader) | 0, 1–127 | OFF, -70 to +10 dB | 0 = OFF; 1-127 = linear -70 dB to +10 dB |
| 8 | Gain | 0–127 | -20 to +60 dB | NOT on Ch USB/BT. Continuous control |
| 9 | Low Cut | 0–127 | 20 to 600 Hz | NOT on Ch USB/BT. Continuous control |
| 10 | Balance / Pan | 0–127 | 1.0 LEFT to 1.0 RIGHT | 64 = 0.0 CENTER |
| 11 | Compressor | 0–100 | 0% to 100% | NOT on Ch USB/BT. Values 101-127 = 100% |
| 12 | Phantom Power (48V) | 0 / 1-127 | OFF / ON | ONLY on Ch 1 + 2 (XLR) |
| 14 | Send Level to Mon1 | 0, 1–127 | OFF, -70 to +10 dB | 0 = OFF; 1-127 = linear -70 dB to +10 dB |
| 15 | Send Level to Mon2 | 0, 1–127 | OFF, -70 to +10 dB | " |
| 16 | Send Level to FX 1 | 0, 1–127 | OFF, -70 to +10 dB | " |
| 17 | Send Level to FX 2 | 0, 1–127 | OFF, -70 to +10 dB | " | |

### 1.3 Bus Channels — Control Change

**Main Bus (Ch 7)**:
| CC# | Parameter | CC Range | Value Range | Notes |
|---|---|---|---|---|
| 7 | Bus Level | 0, 1–127 | OFF, -70 to +10 dB | 0 = OFF; 1-127 = linear -70 dB to +10 dB |
| 10 | Bus Balance | 0–127 | 1.0 LEFT to 1.0 RIGHT | ONLY on Main Bus. 64 = 0.0 CENTER |
| 8 | Bus Limiter | 0–127 | -30 to 0 dB | NOT on FX 1/2 Bus. Continuous control |
| 11–19 | 9-Band EQ (62Hz–16kHz) | 0–127 | -15 to +15 dB | NOT on FX 1/2 Bus. 64 = 0.0 dB (center) |

**Monitor 1 (Ch 8), Monitor 2 (Ch 9)**:
| CC# | Parameter | CC Range | Value Range | Notes |
|---|---|---|---|---|
| 7 | Bus Level | 0, 1–127 | OFF, -70 to +10 dB | 0 = OFF; 1-127 = linear -70 dB to +10 dB |
| 8 | Bus Limiter | 0–127 | -30 to 0 dB | Continuous control |
| 11–19 | 9-Band EQ (62Hz–16kHz) | 0–127 | -15 to +15 dB | 64 = 0.0 dB (center) |

**FX 1 Bus (Ch 10), FX 2 Bus (Ch 11)**:
| CC# | Parameter | CC Range | Value Range | Notes |
|---|---|---|---|---|
| 7 | Bus Level | 0, 1–127 | OFF, -70 to +10 dB | Level only (no limiter, no EQ) |

### 1.4 FX Slot Control — Program Change / Control Change

**FX 1 (Ch 13), FX 2 (Ch 14)** — each FX slot is on its own MIDI channel:
| Message | Parameter | Range | Value Range | Notes |
|---|---|---|---|---|
| Program Change | Effect Preset | 1–16 | Preset 1–16 | PC 0 & 17-127 = ignored |
| CC 1 | Parameter 1 | 0–100 | 0% to 100% | Values 101-127 = identical to 100% |
| CC 2 | Parameter 2 | 0 / 1-127 | Value A / Value B | Switch: 0 = "Value A", 1-127 = "Value B" |

### 1.5 Global Control (Ch 15) — Program Change / CC / Note

All global controls operate on MIDI channel 15 (0-indexed) = channel 16 (1-indexed).

| Message | Parameter | Range | Value Range | Notes |
|---|---|---|---|---|
| Program Change | Load Snapshot | 1–16 | Snapshot 1–15, Reset = 16 | PC 0 & 17-127 = ignored. PC 16 = RESET mixer |
| CC 1 | FX Mute | 0 / 1-127 | NO MUTE / MUTE | Mutes BOTH FX sends. Switch: 0 = mute off, 1-127 = mute on |
| Note On (C-1) | Tap Tempo | Vel 1–127 | 50–250 BPM | Note 0 only. Velocity 0 = ignored. Tempo = interval between hits. Affects both FX slots (global). Only usable for FX 2 delay/echo presets (1-12) |

### 1.6 Channel Capabilities by Input Type

| Feature | Ch 1–2 (XLR) | Ch 3–4 (Combo) | Ch 5–6 (Line) | Ch 7 (USB/BT) |
|---|---|---|---|---|
| Gain | Yes | Yes | Yes | No |
| Compressor | Yes | Yes | Yes | No |
| Low Cut | Yes | Yes | Yes | No |
| Phantom 48V | Yes | No | No | No |
| Pan/Balance | Pan (Mono) | Pan (Mono) | Bal (Stereo) | Bal (Stereo) |
| 4-Band EQ | Yes | Yes | Yes | Yes |
| Sends | Yes | Yes | Yes | Yes |

***

## 2. USB MIDI — SysEx (System Exclusive)

### 2.1 SysEx Identity

* **Manufacturer ID**: `00 20 32` (Behringer / Music Tribe)
* **Model byte**: `21` (FLOW 8 identifier)
* **SysEx dump header**: `F0 00 20 32 21 00 46 4C 4F 57 ...` (starts with "FLOW" in ASCII)

### 2.2 SysEx State Dump

The FLOW 8 supports a full state dump (~3068 bytes) containing all mixer parameters.

**How to trigger**: The dump can only be triggered via BLE using the **0x4B dump trigger** command (see [Section 3.4](#type-0x4b--dump-trigger-3-bytes--confirmed)). It **cannot** be requested via USB MIDI — all USB SysEx probes failed (see below). Our app triggers the dump automatically during BLE connection and on manual/auto sync.

**Tested and failed USB MIDI probes** (none triggered a response):

1. Universal Identity Request: `F0 7E 7F 06 01 F7`
2. `F0 00 20 32 21 00 F7` through `F0 00 20 32 21 7F F7` (various cmd bytes)
3. `F0 00 20 32 00 00 F7` and `F0 00 20 32 00 01 F7`

**Dump reception**: When triggered via BLE, the dump is received on the USB MIDI input port as a SysEx message. This is the mechanism our app uses — BLE triggers the command, USB receives the data.

### 2.3 SysEx Dump Structure

Total size: **3068 bytes** (including F0 start and F7 end).

#### 2.3.1 Data Encoding — 5-Byte Float with Rotating MSB (CONFIRMED)

Parameters are stored as **IEEE 754 32-bit floats** in **little-endian** byte order, packed into 5 SysEx bytes using MSB packing. The MSB byte position **rotates** through 3 modes for consecutive parameters:

**Mode 0 (MSB at position 0):**

```
[MSB] [d0] [d1] [d2] [d3]     bits 0–3 of MSB → d0–d3 bit 7
```

**Mode 1 (MSB at position 2):**

```
[d0] [d1] [MSB] [d2] [d3]     bit 0→d2, bit 1→d3, bit 2→d1, bit 3→d0
```

**Mode 2 (MSB at position 4, offset by +2):**

```
[..2 other bytes..] [MSB] [..gap..] [d0] [d1] [d2] [d3]
                                     bit 2→d0, bit 3→d1, bit 4→d2, bit 5→d3
```

The general rule: `bit_index = data_byte_offset_from_MSB - 1` for positive offsets. The MSB byte may control up to 7 data bytes (spanning multiple parameters).

To decode: restore bit 7 of each data byte from the corresponding MSB bit, then interpret d0–d3 as a little-endian IEEE 754 float.

**Internal float dB range (confirmed via automated calibration with CC#7):**

* **CC=0 (min)**: -144.0 dB (internal representation of "OFF")
* **CC=64 (mid)**: ~-29.7 dB
* **CC=127 (max)**: +10.0 dB

**Verified with automated calibration across all 8 input channels + 4 buses.**

> **Important — SysEx internal range vs MIDI CC range**: The raw SysEx floats store values from -144.0 to +10.0 dB, where -144.0 dB is the mixer's internal representation of silence/OFF. However, the MIDI CC mapping (per the MIDI Implementation Chart) uses a different range: **CC 0 = OFF, CC 1–127 = linear -70 to +10 dB**. The SysEx parser's `db_to_cc` function converts from the internal float to the correct CC value using the -70 to +10 dB range, mapping any internal value below -70 dB to CC 0 (OFF). Similarly, Gain uses **-20 to +60 dB** and Low Cut uses **20 to 600 Hz**.

#### 2.3.2 Dump Layout — Regions

| Region | Offsets | Size | Contents |
|--------|---------|------|----------|
| SysEx header | 0x0000–0x000B | 12 | `F0 00 20 32 21 00 46 4C 4F 57 1A 58` |
| Volatile header | 0x000C–0x0012 | 7 | State hash (0x0D–0x0E constant `4B 09`, rest changes with every parameter modification) |
| Fixed header | 0x0013–0x001F | 13 | Static header data |
| Pre-channel data | 0x0020–0x0066 | 71 | Unknown parameters before first input channel |
| **Ch1 block** | 0x0067–0x00B3 | 77 | Ch1 parameters (Level at **0x0067**, mod0) |
| **Ch2 block** | 0x00B4–0x00F9 | 70 | Ch2 parameters (Level at **0x00B4**, mod1) |
| **Ch3 block** | 0x00FA–0x0146 | 77 | Ch3 parameters (Level at **0x00FA**, mod2) |
| **Ch4 block** | 0x0147–0x0193 | 77 | Ch4 parameters (Level at **0x0147**, mod0) |
| **Ch5-6 block** | 0x0194–0x01D9 | 70 | Ch5-6 parameters (Level at **0x0194**, mod1) |
| **Ch7-8 block** | 0x01DA–0x0226 | 77 | Ch7-8 parameters (Level at **0x01DA**, mod2) |
| **USB-BT block** | 0x0227–0x0337 | 273 | USB-BT parameters (Level at **0x0227**, mod0) |
| **Mon1/Mon2 Region A** | 0x0338–0x038B | 84 | Monitor bus level A (at **0x0338**, mod2) |
| **Mon1/Mon2 Region B** | 0x038C–0x03D1 | 70 | Monitor bus level B (at **0x038C**, mod1) |
| **FX1 block** | 0x03D2–0x03DD | 14 | FX1 3× level floats (contiguous, mod1+mod2+mod0) |
| FX gap | 0x03DE–0x0417 | 58 | Unknown (between FX1 and FX2) |
| **FX2 block** | 0x0418–0x042C | 21 | FX2 3× level floats (mod2+mod0+mod2) |
| Gap | 0x042D–0x04C6 | 154 | Unknown parameters |
| **Main block** | 0x04C7–0x054F | 137 | Main bus parameters (Level at **0x04C7**, mod0) |
| Channel names | 0x0550–0x0607 | 184 | ASCII channel names + per-channel config |
| Slot indices | 0x0608–0x0730 | 297 | Sequential bytes 0x07–0x0F at ~30-byte spacing |
| FX/mixer data | 0x073B–0x07BB | 129 | FX parameters, sparse |
| Bus config | 0x07BC–0x08FF | 324 | Bus parameters, `4D 4C 4C 3F` pattern (limiter?) |
| Bus EQ | 0x0920–0x0BB3 | 660 | 9-band EQ data, `3F 00 7D 00 7A 00 74 01 68 03 50 07` pattern × 6 |
| Global flags | 0x0BB4–0x0BFA | 71 | Boolean flags (`00`/`01`), snapshot config |
| SysEx end | 0x0BFB | 1 | `F7` |

#### 2.3.3 Known Parameter Offsets — Channel Level (Fader)

**CONFIRMED via automated calibration** (CC#7 min/max sweep on all channels with SysEx dump capture).

##### Input Channels

| Channel | MSB Offset | Data Offsets | Mode | Baseline dB | Stride from prev |
|---------|-----------|-------------|------|-------------|-----------------|
| **Ch1** | 0x0067 | 0x0068–0x006B | mod0 | -28.43 | — |
| **Ch2** | 0x00B4 | 0x00B2–0x00B3, 0x00B5–0x00B6 | mod1 | -69.12 | 0x4D (77) |
| **Ch3** | 0x00FA | 0x00FD–0x0100 | mod2 | +10.00 | 0x46 (70) |
| **Ch4** | 0x0147 | 0x0148–0x014B | mod0 | -0.20 | 0x4D (77) |
| **Ch5-6** | 0x0194 | 0x0192–0x0193, 0x0195–0x0196 | mod1 | -11.07 | 0x4D (77) |
| **Ch7-8** | 0x01DA | 0x01DD–0x01E0 | mod2 | -0.35 | 0x46 (70) |
| **USB-BT** | 0x0227 | 0x0228–0x022B | mod0 | -144.00 | 0x4D (77) |

##### Stride Pattern

The encoding mode cycles: **mod0 → mod1 → mod2 → mod0 → ...**

| Transition | Stride | Pattern |
|-----------|--------|---------|
| mod0 → mod1 | 0x4D (77 bytes) | ch1→ch2, ch4→ch5-6, ch7-8→usb-bt |
| mod1 → mod2 | 0x46 (70 bytes) | ch2→ch3, ch5-6→ch7-8 |
| mod2 → mod0 | 0x4D (77 bytes) | ch3→ch4 |
| **3-channel cycle** | **0xE0 (224 bytes)** | ch1→ch4, ch4→usb-bt |

##### Bus Channels

| Bus | MSB Offset(s) | Mode(s) | Regions | Baseline dB | Notes |
|-----|-------------|---------|---------|-------------|-------|
| **Main** | 0x04C7 | mod0 | 1 | -21.50 | Single region, 672 bytes after USB-BT |
| **Mon1/Mon2** | 0x0338, 0x038C | mod2, mod1 | 2 | +4.35 | Dumps idênticos na calibração (estavam linkados por config do usuário; podem ser independentes) |
| **FX1** | 0x03D2, 0x03D9 | mod1+mod2, mod0 | 3 | -10.00 | 3 contiguous floats at 0x03D0–0x03DD |
| **FX2** | 0x0418, 0x041F, 0x0426 | mod2, mod0, mod2 | 3 | -10.00 | 3 floats spaced at 7-byte intervals |

**Bus anomalies:**

* **Mon1/Mon2 produziram dumps idênticos na calibração**: isso ocorreu porque estavam linkados por configuração do usuário no mixer. Quando desvinculados, Mon1 (MIDI ch 9) e Mon2 (MIDI ch 10) controlam regiões independentes. Os offsets exatos de cada um precisam ser re-testados com os monitores desvinculados. As duas regiões (0x0338 e 0x038C) provavelmente correspondem a Mon1 e Mon2 respectivamente.
* **FX1/FX2 have 3 float regions each**: All 3 decode to the same dB value. Likely represent stereo or pre/post-fader copies of the level.

Each channel's parameter block contains Level, Pan, Gain, EQ (4 bands), Compressor, Low Cut, Sends, Mute, Solo, and Phantom Power — all as 5-byte packed floats. The SysEx parser (`src/service/sysex_parser.rs`) decodes all known parameters from the dump.

#### 2.3.4 Channel Names Region (0x0550+)

Channel names are stored as 7-bit ASCII (no MSB packing needed since ASCII is inherently < 0x80). Observed names from a live dump:

| Approx. Offset | Name | Channel |
|----------------|------|---------|
| 0x0554 | "Samson" / "CS1" | Ch1 (XLR) |
| 0x0572 | "Saxophone" | Ch2 (XLR) |
| 0x0590 | "FM3 Left" | Ch3 (Combo) |
| 0x05AF | "FM3 Right" | Ch4 (Combo) |
| 0x05CC | "Synth" | Ch5/6 (Stereo) |
| 0x05E8 | "Karaoke" | Ch7/8 (USB/BT) |

#### 2.3.5 Parameter Data Region — Default Pattern

The parameter area (0x0020–0x04C7) contains a repeating base pattern where most parameters are at their default values:

```
00 00 10 43 00 00 22 10 43 00 00 10 43 08
```

The recurring subsequence `00 00 10 43` encodes the float data bytes of **-144.0 dB** (minimum/off):

* `00 00 10 C3` (with bit 7 of byte\[3] restored from the MSB byte) = IEEE 754 LE float = **-144.0**

The `22` and `08` bytes within the pattern are MSB bytes with specific bits set to restore the high bit of `43` → `C3` on their associated data bytes. When a parameter deviates from the default (e.g., a fader is moved), the corresponding bytes differ from this baseline pattern.

***

## 3. BLE (Bluetooth Low Energy) — Proprietary Protocol

### 3.1 BLE Connection Details

Discovered via HCI snoop log capture on Android (Xiaomi 14 Ultra → FLOW 8).

| Property | Value |
|---|---|
| Device Name | `FLOW 8 LE` |
| MAC Address | `f7:1a:a4:e9:81:8c` (observed, may vary per unit) |
| Phone MAC | `b0:9c:63:96:03:9e` |
| GATT Handle | `0x000b` (single characteristic for all communication) |
| Service UUID | `14839ad4-8d7e-415c-9a42-167340cf2339` |
| Characteristic UUID | `0034594a-a8e7-4b1a-a6b1-cd5243059a57` |
| MTU | Client: 517, Server (FLOW 8): 131 |
| Service Handles | `0x0009..0x000f` |
| GAP Characteristics | Device Name, Appearance, Peripheral Preferred Connection Parameters |

**Important**: The Service UUID is NOT the standard BLE MIDI UUID (`03B80E5A-EDE8-4B33-A751-6CE34EC4C700`). This is a fully proprietary Behringer protocol.

### 3.2 Connection & Authentication Sequence

```
Step 1:  FLOW 8 → Phone:  [0x35] Identity (multi-part, device fingerprint)
Step 2:  GATT service discovery + MTU exchange
Step 3:  FLOW 8 → Phone:  [0x35] Identity re-sent (post-discovery)
Step 4:  Phone → FLOW 8:  [0x39] Auth key (16 bytes — STATIC, see below)
Step 5:  FLOW 8 → Phone:  [0x36] Auth ack ("OK")
Step 6:  Phone → FLOW 8:  [0x37] Session start
Step 7:  FLOW 8 → Phone:  [0x38] Full state dump (4 multi-part chunks)
Step 8:  Phone → FLOW 8:  [0x07] Config request
Step 9:  FLOW 8 → Phone:  [0x27] Snapshot names
Step 10: Phone → FLOW 8:  [0x26] Parameter queries (device name, channel config)
Step 11: FLOW 8 → Phone:  [0x25] Parameter responses
Step 12: Phone → FLOW 8:  [0x21] Context/subscribe command
Step 13: FLOW 8 → Phone:  [0x22] State stream begins (~300ms interval)
```

**CRITICAL DISCOVERY: Authentication key is STATIC and REPLAYABLE.**

The 0x39 auth packet is **identical** across all observed sessions (3+ connections, including reconnects and device reboots):

```
3901fd062b0639f17fe7b7278b8f355a495c2a
```

This is NOT a challenge-response — it is a **fixed key** that the phone sends to the mixer. The mixer simply verifies it and acks. This means we can hardcode this key in our desktop BLE implementation.

The auth is **one-way**: the phone authenticates TO the mixer, not the other way around. The 0x36 ack (`360137`) is also always identical.

### 3.3 Packet Format — Checksum Discovery

**All packets share a common framing structure:**

```
[type:1] [0x01:1] [payload:N] [checksum:1]
```

**Checksum algorithm**: The last byte of every packet is the **sum of all preceding value bytes, modulo 256**.

**Verification (all confirmed):**

| Packet | Value (hex) | Sum of bytes \[0..N-2] | Last byte | Match? |
|--------|------------|----------------------|-----------|--------|
| 1471 | `37 01 38` | 0x37+0x01 = 0x38 | 0x38 | YES |
| 1668 | `26 01 b0 d7` | 0x26+0x01+0xB0 = 0xD7 | 0xD7 | YES |
| 1673 | `26 01 80 a7` | 0x26+0x01+0x80 = 0xA7 | 0xA7 | YES |
| 1759 | `26 01 05 2c` | 0x26+0x01+0x05 = 0x2C | 0x2C | YES |
| 1984 | `4b 01 4c` | 0x4B+0x01 = 0x4C | 0x4C | YES |
| 1464 | `39 01 fd...5c 2a` | sum(bytes\[0..17]) = 0x2A | 0x2A | YES |
| 1648 | `21 01 08...b4 fc` | sum(bytes\[0..17]) = 0xFC | 0xFC | YES |
| 2071 | `21 01 08...f7 b5` | sum(bytes\[0..17]) = 0xB5 | 0xB5 | YES |
| 1301 | `35 02 00 00...11 df` | sum(bytes\[0..18]) = 0xDF | 0xDF | YES |
| 1319 | `35 02 00 01...e5 70` | sum(bytes\[0..9]) = 0x70 | 0x70 | YES |
| 1395 | `35 02 01 00...11 e0` | sum(bytes\[0..18]) = 0xE0 | 0xE0 | YES |
| 1396 | `35 02 01 01...e5 71` | sum(bytes\[0..9]) = 0x71 | 0x71 | YES |
| 1467 | `36 01 37` | 0x36+0x01 = 0x37 | 0x37 | YES |

**Checksum verified on all 13 packets (both directions).** The protocol is NOT encrypted.

### 3.4 Packet Types — Complete Catalog

All packets follow: `[type] [byte1] [data...] [checksum]`.
Checksum = sum of all preceding bytes mod 256.

* **Phone → FLOW 8**: Byte\[1] is always `0x01`
* **FLOW 8 → Phone**: Byte\[1] is `0x01` (data/ack) or `0x02` (identity)
* **Echo pattern**: All Write Requests from phone are echoed back by the mixer as Notifications (acknowledgment)

***

#### Notifications: FLOW 8 → Phone

##### Type 0x35 — Device Identity (multi-part, byte\[1]=0x02)

Sent by the FLOW 8 during connection setup, before authentication. Multi-part message.

```
Part 0 (20 bytes): 35 02 SS 00 7d fc 70 88 95 68 6d 3d 33 54 af b2 4b 4c 11 | chk
Part 1 (11 bytes): 35 02 SS 01 1d 00 00 09 2d e5 | chk
                    ^^^^ ^^^^ ^^ ^^
                    type ver  seq part
```

* Byte\[1] = `0x02` (different from commands which use `0x01`)
* Byte\[2] = sequence number (`00` = first send, `01` = re-send after discovery)
* Byte\[3] = part number (`00` = first half, `01` = second half)
* Combined identity data (21 bytes): `7d fc 70 88 95 68 6d 3d 33 54 af b2 4b 4c 11 1d 00 00 09 2d e5`
* Also sent as a consolidated single packet: `35 01 [21 bytes] [chk]`
* Sent twice: once at initial connection, once after GATT discovery completes
* Identical across all sessions — likely contains device serial/firmware version

##### Type 0x36 — Auth Acknowledgment (3 bytes)

```
36 01 37
```

Zero payload. Always identical. Confirms auth success.

##### Type 0x38 — Full State Dump (multi-part, large payload)

Sent immediately after the 0x37 session start. Contains complete mixer configuration.

```
38 04 02 PP [channel data...]
^^^^ ^^ ^^ ^^
type ?  ?  part
```

Delivered in 4 large chunks (using the full 131-byte MTU). Contains per-channel data including:

* Channel index (0-based: 0x00–0x0d)
* Channel name (length-prefixed UTF-8 string)
* Fader position, EQ settings, sends, routing
* FX parameters, bus configuration

**Observed channel names from dump:**

| Index | Name | Physical Input |
|-------|------|---------------|
| 0x00 | "Samson CS1" | Ch1 (XLR) |
| 0x01 | "Saxophone" | Ch2 (XLR) |
| 0x02 | "FM3 Left" | Ch3 (Combo) |
| 0x03 | "FM3 Right" | Ch4 (Combo) |
| 0x04 | "Synth" | Ch5/6 (Stereo Line) |
| 0x05 | "Karaokê" | Ch7/8 (USB/BT) |
| 0x06+ | (bus config) | Main, Mon1, Mon2, FX |

State dump values change between connections to reflect current mixer state (e.g., fader positions moved between sessions are reflected in the dump).

##### Type 0x27 — Snapshot Names (variable length)

Sent in response to a 0x07 request. Contains names of all snapshot/preset slots.

```
27 01 [len1] [name1] [len2] [name2] ... [padding] [chk]
```

**Observed snapshot names:**

| Slot | Name |
|------|------|
| 1 | "PC Daily" |
| 2 | "Rec Instruments" |
| 3 | "Recording Voice" |
| 4 | "Karaokê" |
| 5 | "4 Mics" |
| 6 | "Karaoke Honda" |

Names are length-prefixed UTF-8 strings. Empty slots have length 0.

##### Type 0x25 — Parameter Query Response (variable length)

Response to a 0x26 parameter query.

```
25 01 PARAM LEN [data...] [chk]
^^^^ ^^^^ ^^^^^ ^^^
type ver  param len
```

**Observed parameter responses:**

| Param | Len | Data | Meaning |
|-------|-----|------|---------|
| 0x05 | 0x0d (13) | `"Abel's Flow 8"` | Device name |
| 0x80 | 0x30 (48) | 12 × 4-byte groups | Channel/EQ config per channel |
| 0xb0 | 0x01 (1) | `00` | Unknown boolean/flag |

##### Type 0x22 — State Stream / Metering (20 bytes, every ~300ms)

The primary continuous notification. **Contains real-time metering data ONLY — fader positions are NOT in this stream.**

```
22 01 M1 00 M2 M3 M4 M5 M6 M7 00 00 MB MB BF BF BF 00 00 [chk]
^^^^ ^^^^ ^^ ^^ ^^^^^ ^^^^^^^^^ ^^^^^ ^^^^^ ^^^^^^^^^ ^^^^^
type ver  |  0  channel metering  0     bus   constant   0
          |                              meter
          main meter?
```

**Confirmed behavior (controlled test with 200+ packets):**

| Bytes | Behavior | Confirmed Role |
|-------|----------|---------------|
| \[2] | Fluctuates 0x34–0x83 | **Metering** — signal level |
| \[3] | Always 0x00 | Unused/zero |
| \[4–5] | Fluctuates 0x00–0x01 | **Metering** — low-level channels |
| \[6–9] | Fluctuates 0x01–0x06 | **Metering** — active channel signals |
| \[10–11] | Always 0x00 | Unused/zero |
| \[12–13] | Fluctuates 0x34–0x70, **always paired** | **Metering** — bus output level (L/R or Main) |
| \[14–16] | **Always `bf bf bf`** (191) | **Constant** — NOT fader positions (unchanged after moving Main to 0) |
| \[17–18] | Always 0x00 | Unused/zero |

**Key finding**: Bytes \[14–16] remain `bf bf bf` even AFTER moving the Main fader to minimum (0x00) via a 0x06 command. This proves they are NOT bus fader positions. They may be a reference level, protocol constant, or metering scale factor.

The 0x22 stream starts ONLY after the phone sends 0x21 context/subscribe commands. Without 0x21, the mixer does not begin streaming.

***

#### Commands: Phone → FLOW 8

##### Type 0x06 — Parameter Change (6 bytes) — CONFIRMED

**The primary command for changing mixer parameters.** Discovered in controlled test (capture 4).

```
06 01 CH PP VV [chk]
^^^^ ^^^^ ^^ ^^ ^^
type ver  ch prm val
```

| Field | Size | Description |
|-------|------|-------------|
| CH | 1 byte | Channel number (1-based) |
| PP | 1 byte | Parameter ID |
| VV | 1 byte | Value (0x00 = min, 0xFF = max) |

**Confirmed parameter: 0x0f = Level (Fader)**

**Observed fader movements (capture 4):**

*Channel 1 fader → maximum (t=499.83s):*

```
06 01 01 0f c7 de   → value 0xc7 (199)
06 01 01 0f d3 ea   → value 0xd3 (211)
06 01 01 0f ec 03   → value 0xec (236)
06 01 01 0f ff 16   → value 0xFF (255) = MAXIMUM
```

*Ch7/8 USB/BT fader → minimum (t=528.10s):*

```
06 01 06 0f 4a 66   → value 0x4a (74)
06 01 06 0f 2a 46   → value 0x2a (42)
06 01 06 0f 00 1c   → value 0x00 (0) = MINIMUM
06 01 06 0f 00 1c   → value 0x00 (repeated, confirming position)
```

**Channel addressing (confirmed):**

| CH byte | Target | Confirmed? |
|---------|--------|-----------|
| 0x01 | Ch1 (XLR input 1) | YES — user moved Ch1 fader |
| 0x02 | Ch2 (XLR input 2) | Inferred |
| 0x03 | Ch3 (Combo input 3) | Inferred |
| 0x04 | Ch4 (Combo input 4) | Inferred |
| 0x05 | Ch5/6 (Stereo Line) | Inferred |
| 0x06 | Ch7/8 (USB/BT input) | YES — user moved USB/BT fader |
| 0x07? | Main Out? | NOT TESTED |
| 0x08? | Mon1? | NOT TESTED |
| 0x09? | Mon2? | NOT TESTED |

**Notes**:

* The FLOW 8 app shows 6 input channel strips (1, 2, 3, 4, 5/6, 7/8) plus a separate Main Out fader. BLE addresses 0x01–0x06 map to the 6 input strips. The Main Out, Mon1, and Mon2 bus fader addresses are not yet confirmed.
* Ch7/8 (0x06) is a **configurable input**: in this test setup it was routed to USB/BT feedback, but in other mixer snapshots it can receive a different stereo input source. The BLE address refers to the channel strip, not the source.

**Echo behavior**: Every 0x06 Write Request is echoed back by the mixer as a Notification with identical data (acknowledgment).

##### Type 0x39 — Authentication Key (19 bytes) — STATIC

```
39 01 fd 06 2b 06 39 f1 7f e7 b7 27 8b 8f 35 5a 49 5c | 2a
^^^^ ^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^   ^^
type ver  16 bytes — STATIC KEY (not a challenge)          chk
```

**CONFIRMED STATIC** across 3+ sessions (including device reboots, reconnects, different capture dates). This is a fixed authentication key, not a challenge-response. Can be hardcoded for BLE implementation.

##### Type 0x37 — Session Start (3 bytes)

```
37 01 38
```

Zero payload. Sent after auth ack. Triggers the 0x38 full state dump from the mixer.

##### Type 0x07 — Config Request (3 bytes)

```
07 01 08
```

Zero payload. Sent after receiving the 0x38 state dump. Triggers the 0x27 snapshot names response.

##### Type 0x21 — Context / Subscribe Command (19 bytes)

Sent to start the state stream and establish the monitoring context.

```
21 01 08 40 41 42 43 c4 c5 c6 cf 00 00 XX XX 00 00 XX [chk]
^^^^ ^^^^ ^^ ^^^^^^^^^^^^^^^^^^^^^^^^^ ^^^^^ ^^^^^^^^^ ^^^
type ver  N  8 constant address bytes   pad   variable   chk
```

* Bytes 3–10: `40 41 42 43 c4 c5 c6 cf` — constant across all sessions. Likely channel/parameter address map.
* Bytes 11–17: Variable portion changes between invocations. May represent current app viewport or subscription filter.
* **Triggers the 0x22 state stream** — the mixer does NOT send 0x22 notifications until it receives at least one 0x21 command.
* The mixer echoes each 0x21 back as a Notification (acknowledgment).

##### Type 0x26 — Parameter Query (4 bytes)

**Correction**: Previously identified as "parameter change" — it is actually a **query/read** command.

```
26 01 PP [chk]
^^^^ ^^^^ ^^
type ver  param
```

Sends a query for parameter PP. The mixer responds with a 0x25 notification containing the data.

**Observed queries:**

| PP byte | Response contains |
|---------|------------------|
| 0x05 | Device name ("Abel's Flow 8") |
| 0x80 | Channel/EQ configuration (48 bytes) |
| 0xb0 | Unknown flag (1 byte: 0x00) |

##### Type 0x4B — Dump Trigger (3 bytes) — CONFIRMED

```
4b 01 4c
```

Zero payload. Triggers a full SysEx dump. The dump is emitted on BOTH the BLE connection AND the USB MIDI output port.

### 3.5 Complete Session Timeline (Capture 4 — Controlled Test)

User actions: Connect → Wait 10s → Ch1 fader to max → Wait 5s → Ch7/8 USB/BT fader to min → Wait 5s → Disconnect → Reconnect → MIDI Dump

```
PHASE 1 — Connection & Full Init (t=409.75–411.36s)
├─ t=409.76  [0x35] Identity part 0+1 ←── FLOW 8
├─ t=410.23  GATT service discovery (Read By Type)
├─ t=410.25  [0x35] Identity re-sent ←── FLOW 8
├─ t=410.77  [0x35] Consolidated identity ←── FLOW 8
├─ t=410.78  [0x39] Auth key ──→ FLOW 8  (STATIC: 3901fd062b06...)
├─ t=410.80  [0x36] Auth ack ←── FLOW 8  (360137)
├─ t=410.85  [0x37] Session start ──→ FLOW 8
├─ t=410.88  [0x38] State dump chunk 1 ←── FLOW 8  (channel config, names)
├─ t=410.90  [0x38] State dump chunk 2 ←── FLOW 8
├─ t=410.91  [0x38] State dump chunks 3-4 ←── FLOW 8
├─ t=411.09  [0x07] Config request ──→ FLOW 8
├─ t=411.33  [0x27] Snapshot names ←── FLOW 8  (6 presets)
├─ t=411.33  [0x26] Query param 0x80 ──→ FLOW 8
├─ t=411.36  [0x25] Response param 0x80 ←── FLOW 8  (channel config)
│
PHASE 2 — Session Start / Subscribe (t=472.80s, ~61s after init)
├─ t=472.81  [0x21] Context command ──→ FLOW 8
├─ t=472.83  [0x21] Echo ←── FLOW 8
├─ t=472.83  [0x21] Context command #2 ──→ FLOW 8
├─ t=472.86  [0x26] Query param 0xb0 ──→ FLOW 8
├─ t=472.89  [0x25] Response param 0xb0 ←── FLOW 8
├─ t=472.89  [0x26] Query param 0x80 ──→ FLOW 8
├─ t=472.92  [0x25] Response param 0x80 ←── FLOW 8
│
PHASE 3 — State Stream Baseline (t=473.13–499.55s, ~87 packets)
├─ t=473.13  First [0x22] state stream packet ←── FLOW 8
├─            0x22 notifications every ~300ms (metering data)
├─            No user interaction — pure baseline metering
│
PHASE 4 — Channel 1 Fader → Maximum (t=499.83s)
├─ t=499.83  [0x06] Ch=0x01 Param=0x0f Val=0xC7 ──→ FLOW 8
├─ t=499.86  [0x06] Echo ←── FLOW 8
├─ t=499.86  [0x06] Ch=0x01 Param=0x0f Val=0xD3 ──→ FLOW 8
├─ t=499.92  [0x06] Echo ←── FLOW 8
├─ t=500.20  [0x06] Ch=0x01 Param=0x0f Val=0xEC ──→ FLOW 8
├─ t=500.22  [0x06] Echo ←── FLOW 8
├─ t=500.22  [0x06] Ch=0x01 Param=0x0f Val=0xFF ──→ FLOW 8  *** MAX ***
├─ t=500.28  [0x06] Echo ←── FLOW 8
│
PHASE 5 — State Stream (Ch1 at max, t=500.5–528.0s, ~54 packets)
├─            0x22 metering continues
│
PHASE 6 — Ch7/8 USB/BT Fader → Minimum (t=528.10s)
├─ t=528.10  [0x06] Ch=0x06 Param=0x0f Val=0x4A ──→ FLOW 8
├─ t=528.12  [0x06] Echo ←── FLOW 8
├─ t=528.12  [0x06] Ch=0x06 Param=0x0f Val=0x2A ──→ FLOW 8
├─ t=528.18  [0x06] Echo ←── FLOW 8
├─ t=528.42  [0x06] Ch=0x06 Param=0x0f Val=0x00 ──→ FLOW 8  *** MIN ***
├─ t=528.45  [0x06] Echo ←── FLOW 8
├─ t=528.45  [0x06] Ch=0x06 Param=0x0f Val=0x00 ──→ FLOW 8  (confirm)
├─ t=528.49  [0x06] Echo ←── FLOW 8
│
PHASE 7 — State Stream Post-USB/BT-Min (t=528.6–550.5s, ~46 packets)
├─            0x22 metering continues (bytes [14-16] still bf bf bf!)
├─ t=550.47  [0x26] Query param 0x05 ──→ FLOW 8
├─ t=550.50  [0x25] Response: "Abel's Flow 8" ←── FLOW 8
│
PHASE 8 — Disconnect (~t=553.8s)
├─            State stream stops, BLE connection drops
│
PHASE 9 — Reconnect (t=598.37s, first attempt failed, second succeeded)
├─ t=598.37  [0x35] Identity ←── FLOW 8  (identical to Phase 1)
├─ t=599.39  [0x39] Auth key ──→ FLOW 8  (IDENTICAL to Phase 1!)
├─ t=599.42  [0x36] Auth ack ←── FLOW 8
├─ t=599.45  [0x37] Session start ──→ FLOW 8
├─ t=599.48  [0x38] State dump ←── FLOW 8  (reflects moved faders)
├─ t=599.51  [0x26] Query param 0x05 ──→ FLOW 8
├─ t=599.55  [0x25] Response: "Abel's Flow 8" ←── FLOW 8
│
PHASE 10 — MIDI Dump (t=620.19s)
├─ t=620.19  [0x4B] Dump trigger ──→ FLOW 8
├─ t=620.24  Write Response ←── FLOW 8
```

### 3.6 Key Observations

1. The FLOW 8 uses **two separate Bluetooth interfaces**:
   * **Bluetooth Classic (A2DP)**: Audio streaming — visible as paired device
   * **BLE GATT**: Control protocol — invisible in Android's paired device list, connects silently

2. The BLE protocol is **fully proprietary**: Custom service UUID, custom binary protocol, not standard BLE MIDI or SysEx.

3. **Simple checksum** (sum mod 256) — confirms the protocol is NOT encrypted.

4. **Authentication is a STATIC KEY** — the 0x39 packet is identical across all sessions. No challenge-response. Can be replayed from desktop.

5. **12 packet types identified**: 0x06, 0x07, 0x21, 0x22, 0x25, 0x26, 0x27, 0x35, 0x36, 0x37, 0x38, 0x4B.

6. **Type 0x06 is the parameter change command**: Format `06 01 CH PARAM VALUE CHK`. Values are 0x00–0xFF (256 steps, wider than MIDI's 128).

7. **Type 0x26 is a parameter QUERY** (not change as previously hypothesized). The mixer responds with Type 0x25.

8. **All commands are echoed**: The mixer echoes every Write Request back as a Notification, providing acknowledgment.

9. **0x21 triggers the state stream**: The mixer does NOT send 0x22 notifications until it receives a 0x21 context command.

10. **0x22 state stream = metering only**: Fader positions are NOT in the stream. They are managed via 0x06 (set), 0x38 (full dump), and 0x26/0x25 (query/response).

11. The SysEx dump over USB is a **side effect** of the 0x4B BLE command.

12. The **reconnect flow is streamlined**: On reconnect, the phone re-sends the same static auth key, receives an updated state dump (reflecting any changes), and can immediately resume control.

***

## 4. Architecture Summary

```
                        USB MIDI (CC/PC, one-way)
┌─────────────┐     ─────────────────────────── →  ┌──────────┐
│  Desktop PC  │     ← ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─   │          │
│  (our app)   │       USB MIDI (SysEx dump)        │  FLOW 8  │
│              │                                     │  Mixer   │
│              │     BLE Proprietary (IMPLEMENTED)   │          │
│              │     ════════════════════════════ ►◄  │          │
└─────────────┘       auth + dump trigger +          │          │
                      snapshot names                  │          │
                                                     │          │
┌─────────────┐     BLE Proprietary                  │          │
│  Phone App   │ ◄════════════════════════════════► │          │
│  (Behringer) │   (bidirectional control)           └──────────┘
└─────────────┘
```

**BLE from desktop is implemented**: The static auth key is hardcoded, the 0x4B dump trigger is used to sync state from the mixer, and snapshot names are fetched via the 0x07/0x27 exchange. The connection is managed by the `btleplug` crate in `src/service/ble.rs`.

> **Known limitation (Windows):** BLE subscribe for snapshot name notifications may fail with `"The attribute cannot be written."` (HRESULT 0x80650003). This is a platform-level issue. Snapshots still load — only names are unavailable.

***

## 5. Confirmed Findings

### Protocol Fundamentals

* \[x] Checksum algorithm: last byte = sum of all preceding bytes mod 256
* \[x] Protocol is NOT encrypted (simple checksum, readable ASCII in payloads)
* \[x] Proprietary service UUID: `14839ad4-8d7e-415c-9a42-167340cf2339`
* \[x] All communication on single GATT handle 0x000b
* \[x] 12 packet types identified (see section 3.4)

### Authentication

* \[x] **Auth key is STATIC and REPLAYABLE**: `3901fd062b0639f17fe7b7278b8f355a495c2a`
* \[x] Verified identical across 3+ sessions (including reboots and reconnects)
* \[x] Auth is one-way: phone → mixer, mixer just acks with `360137`
* \[x] Full auth flow: 0x35 (identity) → 0x39 (static key) → 0x36 (ack) → 0x37 (session start)

### Parameter Control

* \[x] **Type 0x06 = parameter change command**: `06 01 CH PARAM VALUE CHK`
* \[x] Parameter 0x0f = Level/Fader, values 0x00 (min) to 0xFF (max)
* \[x] Channel 0x01 = Ch1, Channel 0x06 = Ch7/8 USB/BT input (confirmed by user)
* \[x] Type 0x26 = parameter QUERY (not change); response via Type 0x25
* \[x] Param 0x05 = device name, Param 0x80 = channel config, Param 0xb0 = flag

### State & Metering

* \[x] Type 0x22 = metering stream (NOT fader positions), every ~300ms
* \[x] Bytes \[14-16] of 0x22 = constant `bf bf bf` (unchanged after fader moves)
* \[x] Type 0x38 = full mixer state dump (channel names, config, fader positions)
* \[x] Type 0x27 = snapshot/preset names (triggered by Type 0x07 request)
* \[x] 0x21 context command triggers the 0x22 state stream

### SysEx Dump Parsing

* \[x] **All channel parameters mapped**: Level, Gain, Pan, Compressor, Low Cut, 4-band EQ, Sends (Mon1, Mon2, FX1, FX2), Mute, Solo, Phantom Power
* \[x] **All bus parameters mapped**: Level, Balance, Limiter, 9-band EQ (Main, Mon1, Mon2)
* \[x] **FX parameters mapped**: Preset, Parameter 1, Parameter 2 (for both FX1 and FX2)
* \[x] **Channel names parsed**: 7-bit ASCII from region 0x0554+
* \[x] **CC range corrections confirmed against MIDI Implementation Chart**: Level/Send = CC 0 OFF, CC 1-127 = -70 to +10 dB; Gain = -20 to +60 dB; Low Cut = 20 to 600 Hz; Comp = 0-100%; Limiter = -30 to 0 dB

### Infrastructure

* \[x] All commands are echoed back by mixer (acknowledgment pattern)
* \[x] **Type 0x4B (`4b 01 4c`) = dump trigger** — confirmed multiple times
* \[x] SysEx dump emitted on both BLE and USB MIDI when triggered
* \[x] **BLE connection implemented from desktop** via `btleplug` crate — auth, dump trigger, snapshot names all working
* \[x] **Windows BLE limitation identified**: Subscribe for snapshot names fails with HRESULT 0x80650003 on some Windows systems

## 6. Open Questions / Future Research

### Resolved

* \[x] ~~Investigate 0x39 auth replayability~~: **CONFIRMED STATIC** — identical across all sessions
* \[x] ~~Decode 0x26 second byte~~: byte\[1] is always `0x01` (protocol version), byte\[2] is the parameter ID being queried
* \[x] ~~Map 0x22 state stream~~: **CONFIRMED as metering only** — fader positions not present
* \[x] ~~Capture more packet types~~: 5 new types found (0x06, 0x07, 0x25, 0x27, 0x38)
* \[x] ~~Decode SysEx float encoding~~: 5-byte packed IEEE 754 floats with rotating MSB position (3 modes)
* \[x] ~~Map ALL channel Level offsets~~: Ch1–USB-BT, Main, Mon1/2, FX1/2 — all confirmed via automated calibration
* \[x] ~~Map remaining SysEx dump byte offsets~~: All known parameters parsed (Level, Gain, Pan, EQ, Comp, LowCut, Sends, Mute, Solo, Phantom, Bus EQ, Limiter, FX Preset/Params, Channel Names)
* \[x] ~~Implement BLE connection from desktop~~: via `btleplug` crate — auth, dump trigger, snapshot names all working

### Open

#### BLE Protocol

* \[ ] **Map Main Out / Mon1 / Mon2 BLE addresses**: CH=0x06 confirmed as Ch7/8 (USB/BT). Need to test Main Out, Mon1, Mon2 faders to find their CH values (likely 0x07, 0x08, 0x09).
* \[ ] **Map all parameter IDs for Type 0x06**: We know 0x0f = Level. What are EQ, Pan, Gain, Sends, Mute, Solo, Compressor, etc.?
* \[ ] **Decode 0x38 BLE state dump structure**: Full byte mapping of the 4-chunk dump to individual mixer parameters
* \[ ] **Decode 0x21 variable bytes**: What do the variable bytes represent? Viewport? Subscription filter?
* \[ ] **Map 0x22 metering bytes to specific channels**: Which bytes correspond to which channels?
* \[ ] **Test EQ/FX/Send changes via BLE**: Do they use 0x06 with different parameter IDs, or different packet types?
* \[ ] **Investigate Windows BLE subscribe failure**: HRESULT 0x80650003 prevents reading snapshot names on some Windows systems. May be a btleplug or OS-level issue.

#### SysEx

* \[ ] **Re-test Mon1/Mon2 unlinked**: Confirm which SysEx region (0x0338 vs 0x038C) maps to Mon1 and which to Mon2 when monitors are not linked in mixer config.

#### General

* \[ ] Determine if firmware updates have changed the protocol or auth key
* \[ ] **Test if the static auth key is per-device or universal across all FLOW 8 units** — The key `3901fd06...` was captured from a single unit. The implementation hardcodes it but is designed to be configurable in the future.
