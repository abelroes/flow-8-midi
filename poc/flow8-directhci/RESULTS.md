# DirectHCI POC — hardware results

| Field | Value |
|-------|-------|
| Date | 2026-10-22 |
| Host / Windows build | Windows host via WSL path (`flow8-directhci-poc3.exe`) |
| BT controller (name, VID:PID) | (not recorded this run) |
| DirectHCI revision / installer | DirectHCI-Setup-V2.0.1 |
| FLOW 8 firmware (if known) | build `0x2DE5` (from HandshakeHost) |
| `client_id` source | `~/.flow8-midi/client_id` = `8f8fe57232762f5c233369d258e8fbbb` |

## Stages

| Stage | Pass? | Notes |
|-------|-------|-------|
| Acquire / WinUSB takeover | yes | DirectHCI service + named pipe |
| Scan FLOW LE | yes | `FLOW 8 LE` random:`F7:1A:A4:E9:81:8C` |
| Connect + GATT discover | yes | `att_mtu=131`; custom service `14839ad4-…`, char `0034594a-…` handle `0x000B` props `0x18` |
| Subscribe (CCCD) | n/a → yes (passive) | No `0x2902`. Write Notify to `value_handle+1` (`0x000C`) → ATT `0x03`. **Approach C:** `listen()` without CCCD |
| HandshakeHost `0x35` | yes | `pairing_any=true` proto=`0x0009` fw=`0x2DE5` |
| HandshakeClient / `0x36` | yes | Full framed `0x39` (19 bytes) on ATT — bare 16-byte payload does **not** get `0x36` |
| MixerState `0x38` | yes | First notify 128 bytes; multi-chunk header `38 04 …` |
| Windows restore (`WindowsOwned`) | (not verified) | Session shutdown logged; confirm Control Panel / `directhci.exe status` if radio stuck |

## Contrast (optional)

| Path | CCCD / subscribe | Reached `0x38`? |
|------|------------------|-----------------|
| Main app (`btleplug` + WinRT) | CCCD write fails (`0x80650003` / Write Not Permitted) | no (blocked before notify) |
| This POC (DirectHCI) | Passive listen (no CCCD on peripheral) | **yes** |

## Go / no-go

- [x] **Go** — consider Windows HCI backend behind a feature flag
- [ ] **No-go** — reason:

**Findings for product path**

1. FLOW 8 GATT has **no CCCD**; Notify still arrives once a passive ATT listener is active (WinRT CCCD write is a dead end, not a DirectHCI-only quirk).
2. HandshakeClient must be the **framed 19-byte `0x39`** packet (matches HCI captures), not payload-only 16 bytes (`ble_session::write_raw` strip looks wrong for this device).

## Logs (paste key lines)

```
[POC] Approach C: ATT 0x03 on 0x000C … passive listen without CCCD
[POC] HandshakeHost pairing_any=true proto=0x0009 fw=0x2DE5
[POC] Writing HandshakeClient 0x39 (19 bytes framed)…
[POC] MixerState (0x38) received — 128 bytes
[POC] hex=38 04 01 00 00 02 E6 7F 00 24 7F 7F 7F 7F 7F 00 …
[POC] SUCCESS — MixerState (0x38) received
```
