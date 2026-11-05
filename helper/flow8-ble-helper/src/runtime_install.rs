//! Windows install step (runs elevated with the helper UAC prompt).
//!
//! When the DirectHCI service is not registered, deploy the same layout as the
//! end-user Setup — both products from the unified build output next to this helper:
//!
//!   {ProgramFiles}\FLOW 8 MIDI Controller\  flow-8-midi.exe, flow8-ble-helper.exe
//!   {ProgramFiles}\DirectHCI\               DirectHCI runtime + libwdi.dll + service

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SERVICE_NAME: &str = "DirectHCI";
const APP_DIR_NAME: &str = "FLOW 8 MIDI Controller";
const DIRECTHCI_DIR_NAME: &str = "DirectHCI";

const APP_FILES: &[&str] = &["flow-8-midi.exe", "flow8-ble-helper.exe"];
const DIRECTHCI_FILES: &[&str] = &[
    "directhcid.exe",
    "directhci.exe",
    "directhci-control-panel.exe",
    "directhci-ble.exe",
];

/// Install Flow 8 + DirectHCI under Program Files if DirectHCI is not available.
pub fn ensure_windows_install() -> Result<(), String> {
    if service_installed()? {
        eprintln!("[flow8-ble-helper] DirectHCI service present — skip install");
        return Ok(());
    }

    let source = locate_build_payload()?;
    let pf = program_files()?;
    let app_dest = pf.join(APP_DIR_NAME);
    let dhci_dest = pf.join(DIRECTHCI_DIR_NAME);

    eprintln!(
        "[flow8-ble-helper] DirectHCI not installed — installing Flow 8 + DirectHCI from {}",
        source.display()
    );

    install_files(&source, &app_dest, APP_FILES)?;
    install_files(&source, &dhci_dest, DIRECTHCI_FILES)?;
    ensure_libwdi(&source, &dhci_dest)?;

    let daemon = dhci_dest.join("directhcid.exe");
    let status = Command::new(&daemon)
        .arg("install-service")
        .status()
        .map_err(|e| format!("run install-service: {e}"))?;
    if !status.success() {
        return Err(format!(
            "directhcid install-service failed (exit {:?}) for {}",
            status.code(),
            daemon.display()
        ));
    }

    eprintln!(
        "[flow8-ble-helper] installed:\n  {}\n  {} (+ DirectHCI service)",
        app_dest.display(),
        dhci_dest.display()
    );
    Ok(())
}

fn program_files() -> Result<PathBuf, String> {
    std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .ok_or_else(|| "ProgramFiles env var missing".into())
}

/// Unified cargo / stage folder: app, helper and DirectHCI EXEs side by side.
fn locate_build_payload() -> Result<PathBuf, String> {
    let helper = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let dir = helper
        .parent()
        .ok_or("helper has no parent directory")?
        .to_path_buf();

    for name in APP_FILES.iter().chain(DIRECTHCI_FILES.iter()) {
        if !dir.join(name).is_file() {
            return Err(format!(
                "build payload incomplete: missing {name} in {} \
                 (build Flow 8 + sibling DirectHCI into one CARGO_TARGET_DIR)",
                dir.display()
            ));
        }
    }
    Ok(dir)
}

fn install_files(source: &Path, dest: &Path, names: &[&str]) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    for name in names {
        let from = source.join(name);
        let to = dest.join(name);
        fs::copy(&from, &to).map_err(|e| format!("copy {name} → {}: {e}", dest.display()))?;
    }
    Ok(())
}

fn ensure_libwdi(source_dir: &Path, dest: &Path) -> Result<(), String> {
    let dest_dll = dest.join("libwdi.dll");
    if dest_dll.is_file() {
        return Ok(());
    }

    let mut candidates = vec![
        source_dir.join("libwdi.dll"),
        source_dir.join("libwdi").join("libwdi.dll"),
    ];
    if let Some(local) = dirs::data_local_dir() {
        candidates.push(local.join("DirectHCI").join("native").join("libwdi.dll"));
    }

    for from in candidates {
        if !from.is_file() {
            continue;
        }
        fs::copy(&from, &dest_dll)
            .map_err(|e| format!("copy libwdi.dll from {}: {e}", from.display()))?;
        eprintln!(
            "[flow8-ble-helper] libwdi.dll ← {}",
            from.display()
        );
        return Ok(());
    }

    Err(
        "libwdi.dll not found (required next to DirectHCI for Bluetooth prepare). \
         Build once: DirectHCI\\scripts\\windows\\build-libwdi.ps1 \
         → %LOCALAPPDATA%\\DirectHCI\\native\\libwdi.dll"
            .into(),
    )
}

fn service_installed() -> Result<bool, String> {
    let status = Command::new("sc.exe")
        .args(["query", SERVICE_NAME])
        .status()
        .map_err(|e| format!("sc.exe query: {e}"))?;
    Ok(status.success())
}
