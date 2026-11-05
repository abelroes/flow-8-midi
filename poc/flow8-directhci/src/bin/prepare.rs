//! CLI helper: Prepare Controller via DirectHCI IPC (no GUI).
//!
//! `directhci.exe` has no prepare subcommand — only the Control Panel does.
//! This binary calls the same IPC as the panel with explicit trust consent.

use std::env;
use std::process::ExitCode;

use directhci_client::DirectHciClient;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    #[cfg(windows)]
    let _presence = directhci_windows::process_lifecycle::enter_application()
        .map_err(|e| format!("process lifecycle: {e}"))?;

    #[cfg(not(windows))]
    {
        return Err("prepare helper runs on Windows only".into());
    }

    let mut args = env::args().skip(1);
    let mut controller_id: Option<String> = None;
    let mut trust = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trust-certificate" => trust = true,
            "--controller" | "-c" => {
                controller_id = Some(
                    args.next()
                        .ok_or("--controller requires an id")?,
                );
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }

    if !trust {
        return Err(
            "refusing to prepare without --trust-certificate \
             (libwdi installs a one-time signing cert to LocalMachine Root/TrustedPublisher)"
                .into(),
        );
    }

    let client = DirectHciClient::connect_or_start(
        "flow8-directhci-prepare",
        Some(env!("CARGO_PKG_VERSION").into()),
    )
    .map_err(|e| format!("connect/start DirectHCI: {e}"))?;

    let id = match controller_id {
        Some(id) => id,
        None => {
            let list = client
                .list_controllers()
                .map_err(|e| format!("list controllers: {e}"))?;
            match list.as_slice() {
                [one] => one.id.clone(),
                [] => return Err("no USB Bluetooth controllers found".into()),
                many => {
                    eprintln!("multiple controllers — pass --controller <id>:");
                    for c in many {
                        eprintln!("  {}", c.id);
                    }
                    return Err("controller_id required".into());
                }
            }.to_string()
        }
    };

    let status = client
        .controller_preparation_status(&id)
        .map_err(|e| format!("preparation status: {e}"))?;
    if status.ready {
        println!("already ready: {id}");
        let _ = client.set_preferred_controller(&id);
        return Ok(());
    }

    println!("preparing controller {id} (trust acknowledged)…");
    let prepared = client
        .prepare_controller(&id, true)
        .map_err(|e| format!("prepare_controller: {e}"))?;

    println!(
        "prepared: id={} ready={} already_prepared={} hw={}",
        id,
        prepared.status.ready,
        prepared.already_prepared,
        prepared.status.hardware_id
    );

    if !prepared.status.ready {
        return Err(format!(
            "prepare finished but not Ready: {:?}",
            prepared.status
        ));
    }

    client
        .set_preferred_controller(&id)
        .map_err(|e| format!("set preferred controller: {e}"))?;
    println!("preferred controller set to {id}");
    Ok(())
}

fn print_help() {
    println!(
        "flow8-directhci-prepare — Prepare a Bluetooth controller for DirectHCI\n\n\
         Usage:\n  \
           flow8-directhci-prepare --trust-certificate [--controller <id>]\n\n\
         Requires Administrator. Starts the DirectHCI service if needed.\n\
         With one controller, --controller is optional."
    );
}
