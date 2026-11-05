# BLE Protocol Map — FLOW 8

Protocol reference for the FLOW 8 mixer BLE communication, derived from static analysis of the official APK (`com.musicgroup.xairbt` v1.12).

| File | Contents |
|------|----------|
| [ble-wire-map.md](./ble-wire-map.md) | Complete protocol spec: GATT, framing, field types, commands, MixerState, auth, pairing, settings, metering |
| [data/xairbt_cmd_catalog.json](./data/xairbt_cmd_catalog.json) | Machine-readable command catalog (51 CommandTypes + field tables) |
| [data/uf8_scale3_lut.json](./data/uf8_scale3_lut.json) | uf8 scale-3 LUT (256 floats: fader/send dB mapping) |
