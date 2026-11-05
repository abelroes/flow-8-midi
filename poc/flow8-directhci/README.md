# FLOW 8 DirectHCI POC

Disposable Windows spike: talk to the FLOW 8 over **raw HCI** via [DirectHCI](https://github.com/NanamiKite/DirectHCI), bypassing WinRT CCCD (`0x80650003`).

**Success:** print `MixerState (0x38) received` and exit 0, then restore Windows Bluetooth ownership.

This does **not** change the main `flow-8-midi` BLE path (`btleplug` / WinRT).

## Automated setup (preferred)

Script: `scripts/Install-DirectHci.ps1` — silent install → start service → wait for pipe → Prepare Controller.

Place `DirectHCI-Setup-0.1.0-alpha.1.exe` next to the script (or pass `-SetupPath`). Build the prepare helper once:

```powershell
cd \\wsl.localhost\kali-linux\home\abel\projects\flow-8-midi\poc\flow8-directhci
cargo build --release --bin flow8-directhci-prepare
```

Elevated PowerShell:

```powershell
cd \\wsl.localhost\kali-linux\home\abel\projects\flow-8-midi\poc\flow8-directhci\scripts
# copy/setup EXE into this folder first
Set-ExecutionPolicy -Scope Process Bypass
.\Install-DirectHci.ps1
```

Then run the POC (still elevated):

```powershell
cd ..
.\target\release\flow8-directhci-poc.exe
```

**Note:** `directhci.exe` has no `prepare` CLI. The script uses `flow8-directhci-prepare.exe`, which calls the same IPC as the Control Panel (`prepare_controller` + `--trust-certificate`).

## Prerequisites (manual alternative)

1. **Wired keyboard/mouse** — DirectHCI temporarily takes the PC Bluetooth radio.
2. Install **DirectHCI** `0.1.0-alpha.1` (Control Panel + service). Only Intel AX201 is documented upstream.
3. Open **DirectHCI Control Panel** (Admin) → **Start Service** → select controller → **Prepare Controller** until **Ready**.
4. Run this POC **elevated** (Admin). Acquire requires elevation.
5. Sibling checkout of DirectHCI for the path dependency:

```text
/home/abel/projects/DirectHCI          ← git clone
/home/abel/projects/flow-8-midi/poc/flow8-directhci
```

If DirectHCI lives elsewhere, edit the `path =` entries in `Cargo.toml`.

6. FLOW 8 powered on and advertising as **FLOW LE**.

## Build (Windows)

**Prefer a native Windows toolchain** (MSVC recommended; matches DirectHCI’s own docs). From this directory on Windows:

```powershell
cargo build --release
```

Cross-compile from WSL (`x86_64-pc-windows-gnu`) currently fails inside DirectHCI (`windows::core::Error::from_win32` removed in windows 0.62). Build on Windows instead.

Tested path dependency against DirectHCI checkout at `../../../DirectHCI` (pin the SHA you validate in `RESULTS.md`).

## Run

Elevated PowerShell:

```powershell
cd poc\flow8-directhci
cargo run --release
# or
.\target\release\flow8-directhci-poc.exe
```

Multiple controllers:

```powershell
.\flow8-directhci-poc.exe --controller <id>
```

IDs: `& "$env:ProgramFiles\DirectHCI\directhci.exe" controllers`

## What it does

1. Acquire controller via DirectHCI (WinUSB takeover)
2. Scan → connect FLOW LE
3. GATT discover → **subscribe** (CCCD write)
4. Wait `0x35` → write 16-byte `client_id` → wait `0x36` (pairing button if needed)
5. Write framed `0x36` ack + `0x37` → wait first notify with `0x38`
6. Disconnect → Windows Bluetooth restored

Prefer `~/.flow8-midi/client_id` when present (same file as the main app).

## Recovery

If the session dies mid-run and Bluetooth stays broken:

- DirectHCI Control Panel → stop/start service, or follow upstream [recovery](https://github.com/NanamiKite/DirectHCI/blob/master/docs/installation.md#recovery)
- `directhci.exe status` / `doctor`

## Results

Fill `RESULTS.md` after a hardware run (go/no-go for a product Windows HCI backend).

## License

GPL-3.0-only (aligned with DirectHCI and flow-8-midi).
