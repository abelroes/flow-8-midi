# BLE Wire Map — FLOW 8

Protocol of the FLOW 8 mixer BLE communication.  
Machine catalog: [`data/xairbt_cmd_catalog.json`](./data/xairbt_cmd_catalog.json) · Fader LUT: [`data/uf8_scale3_lut.json`](./data/uf8_scale3_lut.json)

---

## 1. GATT layer

| Item | Value |
|------|-------|
| Service UUID | `14839ad4-8d7e-415c-9a42-167340cf2339` |
| Characteristic UUID | `0034594a-a8e7-4b1a-a6b1-cd5243059a57` |
| Device name | `FLOW 8` |
| Direction | Write (host→device), Notify (device→host) — same characteristic |
| Transport | Binary frames (§2). A hex-ASCII debug mode exists in the library but is not used in production. |

---

## 2. Packet framing

### 2.1 Single chunk (`total_chunks == 1`)

```
[0]      type          u8   CommandType
[1]      total_chunks  u8   = 0x01
[2..n-2] payload       …    serialized fields (§3)
[n-1]    checksum      u8   sum(bytes[0..n-2]) mod 256
```

### 2.2 Multi-chunk (`total_chunks > 1`)

Used for large payloads (`0x38` MixerState, data transfers).

```
[0] type
[1] total_chunks
[2] seq_id          (fragmented message id)
[3] chunk_index     (0..total-1)
[4..] payload_slice
[last] checksum     sum of all preceding bytes mod 256
```

---

## 3. Field serialization

Each command has a field descriptor table. Fields are serialized in table order.

### 3.1 Wire types

| Type | Name | Bytes | Notes |
|------|------|-------|-------|
| 1 / 2 / 3 | `u8` | 1 | 1=slot, 2=channel id, 3=generic byte |
| 4 | `u16be` | 2 | Big-endian |
| 5 | `float→uf8` | 1 | `xairbt_float_to_uf8(scale, f32)` → u8 |
| 6 | `bool bitpack` | 0 or 1 | Accumulates bits; flushes every 8 bools |
| 7 | `string` | 1+len | `u8 length` + bytes (no NUL); max 255 |
| 8 | `blob` | 2+len | `u8 tag` + `u8 len` + `len` bytes (Setting) |
| 9 | `u32be` | 4 | |
| 10 | `struct` | Σ children | Nested once |
| 11 | `array` | count × child | Nested repeated |

### 3.2 `float→uf8` scales

| Scale | Use | Encode | Decode |
|-------|-----|--------|--------|
| 1 | Pan / balance | `round(x*127 + 127)`, clamp [-1,1] | `(uf8-127)/127` |
| 2 | Squeezer 0..1 | `round(x*255)`, clamp [0,1] | `uf8/255` |
| 3 | Fader / send dB | 256-entry LUT (−144 … +10 dB) | index → LUT |
| 4 | PEQ/GEQ gain dB | `(x/15)*127 + 127`, clamp [-15,15] | inverse |
| 5 | Gain / limiter ±60 dB | `(x+60)*2`, clamp [-60,60] | inverse |
| 6 | PEQ Q | dedicated helper | dedicated helper |

Scale-3 LUT: `data/uf8_scale3_lut.json` (256 f32 values).

---

## 4. Channel IDs

| ID | Name |
|----|------|
| 0 | InputChannel1 |
| 1 | InputChannel2 |
| 2 | InputChannel3 |
| 3 | InputChannel4 |
| 4 | InputChannel5 |
| 5 | InputChannel6 |
| 6 | InputChannelBTUSB |
| 0x0a | AUXBusChannel1 (Mon1) |
| 0x0b | AUXBusChannel2 (Mon2) |
| 0x0c | FXBusChannel1 |
| 0x0d | FXBusChannel2 |
| 0x0f | LRBusChannelId (Main) |

Input IDs are **0-based** (Ch1 = `0x00`).

---

## 5. Command catalog

`dir`: in = device→host, out = host→device, inout = both.  
Payload = fields only (§3), after `[type][chunks]`.

### 5.1 Channel controls (inout)

| Type | Name | Payload | Scales |
|------|------|---------|--------|
| `0x00` | ChannelPan | `id:u8`, `value:uf8` | scale 1 |
| `0x01` | ChannelSolo | `id:u8`, `value:bool` | bitpack |
| `0x02` | ChannelGain | `id:u8`, `db:uf8` | scale 5 |
| `0x03` | ChannelGEQ | `id`, `geq_id`, `freq:u16be`, `q:uf8`, `gain:uf8` | q=6, gain=4 |
| `0x04` | ChannelHPF | `id`, `on:bool`, `freq:u16be` | |
| `0x05` | ChannelLabel | nested: `id:u8`, `icon:u16be`, `text:string` | |
| `0x06` | ChannelMaster | `src:u8`, `dst:u8`, `db:uf8` | scale 3 |
| `0x08` | ChannelMute | `id:u8`, `value:bool` | |
| `0x09` | ChannelPEQ | `id`, `peq_id`, `freq:u16be`, `q:uf8`, `gain:uf8` | q=6, gain=4 |
| `0x10` | ChannelPhase | `id`, `value:bool` | |
| `0x13` | ChannelSqueezer | `id`, `value:uf8` | scale 2 |
| `0x14` | ChannelLimiter | `id`, `db:uf8` | scale 5 |
| `0x15` | ChannelPhantomPower | `id`, `value:bool` | |
| `0x41` | OutputChannelSelect | `id:u8` | |
| `0x49` | ChannelReserved | `id`, `value:bool` | |
| `0x4a` | ChannelDelay | `id`, `val:u32be` | |

### 5.2 FX

| Type | Name | Payload |
|------|------|---------|
| `0x11` | FXSetup | `id`, `val1..3:u8`, `enable_main/mon1/mon2:bool×3` |
| `0x30` | FXChannelState | nested (= FX schema in MixerState) |
| `0x31` | FXLoadPreset | `id:u8`, `preset:u8` |
| `0x40` | FXTempo | `bpm:u16be` |
| `0x47` | SetFXPresetDesc | `id`, `preset_id`, name/param strings… |
| `0x48` | SetFXPresetDescAck | (in) empty |

### 5.3 Snapshots

| Type | Name | dir | Payload |
|------|------|-----|---------|
| `0x07` | GetSnapshotList | out | empty → response `0x27` |
| `0x27` | SnapshotList | in | snapshot array |
| `0x19` | SnapshotSave | inout | `slot:u8`, nested `info` |
| `0x20` | SnapshotLoad | inout | `slot:u8` |
| `0x12` | SnapshotDelete | inout | `slot:u8` |
| `0x32` | SnapshotRename | inout | `slot:u8`, nested `info` |

### 5.4 Settings

| Type | Name | dir | Payload |
|------|------|-----|---------|
| `0x25` | Setting | inout | type-8 blob: `tag:u8` + `len:u8` + data (§5.4.1) |
| `0x26` | GetSetting | out | `id:u8` (= tag) → response `0x25` |
| `0x23` | GetChannelLabels | out | empty → `0x24` |
| `0x24` | ChannelLabels | in | `num_labels:u8` + label array |
| `0x29` | FactoryReset | out | empty |

#### 5.4.1 Setting tags

| Tag | Meaning | Value |
|-----|---------|-------|
| `0x01` | BT/USB switch mode | 1 byte bool |
| `0x02` | Phones only | 1 byte bool |
| `0x03` | Foot switch mode | 1 byte bool |
| `0x05` | Device name | string (len + UTF-8) |
| `0x07` | USB streaming mode | 1 byte bool |
| `0x08` | Mon1/2 routing | 1 byte |
| `0x09` | Ch5–6 ↔ USB 1–2 | 1 byte bool |
| `0x0a` | Ch7–8 ↔ USB 3–4 | 1 byte bool |
| `0x0b` | Mon stereo link | 1 byte bool |
| `0x0c` | HP use Mon | 1 byte bool |
| `0x0d` | HP use Post | 1 byte bool |
| `0x0e` | Main −10 dBV pad | 1 byte bool |
| `0x0f` | Mon −10 dBV pad | 1 byte bool |
| `0x10` | Snapshot scope bitset | 1 byte |
| `0x11` | Monitor setting | 1 byte bool |
| `0x80` | ChannelStates | blob len `0x30` — 12×4 B (§5.4.2) |
| `0xb0` | Client linked with mixer | 1 byte bool |

#### 5.4.2 Setting `0x80` — ChannelStates

12 mixer slots (7 inputs + 3 outputs + 2 FX), 4 bytes each:

```
for i in 0..11:
  [i*4 + 0]  flags:u8       bit0 = isLeftOn, bit1 = isRightOn
  [i*4 + 1]  preset_lo  ┐
  [i*4 + 2]  preset_hi  ┘   i16le selectedPresetId
  [i*4 + 3]  channel_type:u8
```

### 5.5 Metering

| Type | Name | dir | Payload |
|------|------|-----|---------|
| `0x21` | MeterRequest | out | `num_channels:u8` + up to 15× `channel_id:u8` |
| `0x22` | MeterUpdate | in | 15× `uf8` scale 3 + `gr_values:u16be` |

Meter index → mixer channel mapping:

| Meter index | Mixer index | Peak |
|-------------|-------------|------|
| 0..3 | 0..3 | left |
| 4 | 4 | left |
| 5 | 4 | **right** |
| 6 | 5 | left |
| 7 | 5 | **right** |
| 8 | 6 | left |
| 9 | 6 | left |
| 10..14 | skipped | — |

Right-channel indices: **5, 7** (11 mapped to −1, effectively unused).

`gr_values` (u16be): 2-bit gain reduction per mixer index. For index ≤ 6: bits at `index*2`; else bits at 14.

Default meter request IDs: `40 41 42 43 c4 c5 c6 8f`.

### 5.6 Mixer state

| Type | Name | dir | Payload |
|------|------|-----|---------|
| `0x16` | GetChannelState | out | `id:u8` |
| `0x17` | InputChannelState | inout | nested input (§6.1) |
| `0x18` | OutputChannelState | inout | nested output (§6.2) |
| `0x37` | GetMixerState | inout | empty (request) |
| `0x38` | MixerState | inout | full dump (§6) |

### 5.7 Session / handshake / data / MIDI

| Type | Name | dir | Payload |
|------|------|-----|---------|
| `0x33` | ChannelConnectionState | in | `id`, `sub_channel`, `connected:bool` |
| `0x34` | ChannelSimulateConnectionState | in | same |
| `0x35` | HandshakeHost | in | `id[16]`, `pairing_any:bool`, `protocol_version:u16be`, `firmware_buildnr:u16be` |
| `0x36` | HandshakeReply | inout | empty — ack only |
| `0x39` | HandshakeClient | out | `client_id[16]` |
| `0x42` | RequestData | inout | `name:string`, `stream:u8`, `ack_interval:u8` |
| `0x43` | TransferData | inout | `total_size:u16be`, `pos:u16be`, `stream`, `data[]` |
| `0x44` | AckData | inout | `pos:u16be`, `stream:u8` |
| `0x45` | SetMIDI | inout | values array |
| `0x46` | GetMIDI | out | empty |
| `0x4b` | SysexMidiDump | inout | empty — triggers SysEx export (not primary UI sync) |

#### 5.7.1 Authentication (client ID)

1. Host generates a random **16-byte client ID** and persists it. Same ID across reconnects.
2. Device → host: `HandshakeHost` (`0x35`) with device identity, `pairing_any`, protocol version, firmware build.
3. Host → device: `HandshakeClient` (`0x39`) with the persisted client ID.
4. Device → host: `HandshakeReply` (`0x36`) — ack.
5. Host auto-queues `GetMixerState` (`0x37`).

Frame example:

```
39 01 fd062b0639f17fe7b7278b8f355a495c 2a
│  │  │                                └─ checksum
│  │  └─ client_id[16]
│  └─ chunks = 0x01
└─ type = 0x39
```

#### 5.7.2 Pairing policy (device firmware)

The `pairing_any` flag in `0x35` reflects the mixer pairing button state:

| `pairing_any` | Mixer button | Effect |
|---------------|--------------|--------|
| `0` (false) | not pressed | Device accepts only **known** client IDs |
| `1` (true) | pressed | Device accepts **any** client ID |

Verified behavior (HCI captures):

| Scenario | Result |
|----------|--------|
| Known client ID, button not pressed | ✅ `0x36` + `0x38` received |
| New client ID, button not pressed | ❌ No `0x36`, no `0x38` |
| New client ID, button pressed | ✅ `0x36` + `0x38` received |

**Implementation rules:**
1. First connection: prompt user to press the mixer pairing button. Generate and persist a random 16-byte ID.
2. Subsequent connections: reuse the persisted ID — accepted silently.
3. Read `pairing_any` from `0x35` for UI feedback ("press pairing button").
4. If persisted ID is lost: user must press the pairing button again.

---

## 6. MixerState schema (`0x38`)

Multi-chunk. Arrays in field-table order:

| Field | Count | Child |
|-------|-------|-------|
| `input_channels` | 7 | §6.1 |
| `output_channels` | 3 | §6.2 |
| `fx_channels` | 2 | §6.3 |

After the arrays:

| Field | Type | Scale |
|-------|------|-------|
| `headphone_volume_db` | uf8 | 3 |
| `btusb` | bool | |
| `phones_only` | bool | |
| `foot_pedal_switch` | bool | |
| `mute_inputs` | bool | |
| `usb_streaming_mode` | bool | |
| `ch_56_usb_12` | bool | |
| `ch_78_usb_34` | bool | |
| `mon_stereo_link` | bool | |
| `hp_use_mon` | bool | |
| `hp_use_post` | bool | |
| `main_10dbv_pad` | bool | |
| `mon_10dbv_pad` | bool | |

Full trailing-flag list: see JSON (`MixerState` fields[3:]).

### 6.1 Input channel (×7)

Wire order (bools bit-packed):

1. `id:u8`
2. bools: `muted`, `hpf_on`, `phase`, `phantom_power`, `connected_l`, `connected_r`, `solo`, `reserved`
3. `gain:uf8` scale 5
4. `squeezer:uf8` scale 2
5. `hpf_frequency:u16be`
6. `balance:uf8` scale 1
7. `peq_gain[4]:uf8` scale 4
8. `peq_freq[4]:u16be`
9. `peq_q[4]:uf8` scale 6
10. label nested: `id`, `icon:u16be`, `text:string`

### 6.2 Output channel (×3) — Main / Mon1 / Mon2

1. `id:u8`
2. bools: `muted`, `solo`
3. `vol:uf8` scale 3
4. `limiter:uf8` scale 5
5. `pan:uf8` scale 1
6. `input_channel_gains[7]:uf8` scale 3
7. `geq_gain[9]`, `geq_freq[9]:u16be`, `geq_q[9]`
8. `delay:u32be`

### 6.3 FX channel (×2)

1. `id:u8`
2. bools: `muted`, `enable_main`, `enable_mon1`, `enable_mon2`
3. `vol:uf8` scale 3, `pan:uf8` scale 1
4. `val1..3:u8`, `preset:u8`
5. `input_channel_gains[7]:uf8`, `aux_channel_gains[2]:uf8`

---

## 7. Session flow

```
connect GATT + enable CCCD notify
  → receive HandshakeHost (0x35)
  → send HandshakeClient (0x39) with persisted client_id[16]
  → receive HandshakeReply (0x36)
  → auto-send GetMixerState (0x37)
  → receive MixerState (0x38) [multi-chunk]
  → send MeterRequest (0x21) → receive MeterUpdate (0x22) continuous
  → optional GetSetting (0x26) / Setting (0x25)
  → UI writes (0x06/0x08/…) and labels (0x05)
```

`0x4b` (SysexMidiDump) is for dump/export only, not primary state sync.

---

## 8. Encode examples

| Action | Frame hex |
|--------|-----------|
| Mute Ch1 on | `08 01 00 01 <chk>` |
| Pan Ch1 center | `00 01 00 7F <chk>` |
| Master Ch1→LR | `06 01 00 0F <uf8_s3> <chk>` |
| Gain Ch1 | `02 01 00 <uf8_s5> <chk>` |
| GetMixerState | `37 01 38` |
| MeterRequest 7 inputs | `21 01 07 00 01 02 03 04 05 06 <chk>` |

---

## 9. UI ranges

| Param | Min | Max | Notes |
|-------|-----|-----|-------|
| Master / meter dB | −144 | +10 | uf8 scale 3 |
| Gain dB | −20 | +60 | uf8 scale 5 (wire clamps ±60) |
| Pan | −1 | +1 | uf8 scale 1 |
| HPF Hz | 20 | 600 | u16be |
| Comp (squeezer) | 0 | 1 | uf8 scale 2 |
| Limiter dB | −30 | 0 | uf8 scale 5 |
| GEQ/PEQ gain dB | −15 | +15 | uf8 scale 4 |

### Band indices

- **GEQ** `geq_id` 0..8 → 63, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz
- **PEQ** `peq_id` 0..3 → Low, LowMid, HighMid, High
