//! Elevated Windows BLE helper for flow-8-midi (DirectHCI).
//!
//! Line-delimited JSON over `\\.\pipe\Flow8MidiBle\v1`.
//! Requires Administrator (UAC). Prepares the controller, then runs the FLOW 8 session.

#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

mod ipc;
mod prepare;
mod runtime_install;
mod session;
mod wire;

use std::env;
use std::process::ExitCode;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use ipc::{decode_line, encode_line, hex_decode, hex_encode, ClientMsg, ServerMsg, PIPE_NAME};

fn main() -> ExitCode {
    match run_blocking() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[flow8-ble-helper] FAILED: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_blocking() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        return Err(
            "flow8-ble-helper runs on Windows only (DirectHCI named pipe + WinUSB).".into(),
        );
    }

    #[cfg(windows)]
    {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {e}"))?;
        rt.block_on(run())
    }
}

#[cfg(windows)]
async fn run() -> Result<(), String> {
    let _presence = directhci_windows::process_lifecycle::enter_application()
        .map_err(|e| format!("process lifecycle: {e}"))?;

    let args = parse_args()?;
    if !args.trust {
        return Err(
            "refusing to run without --trust-certificate \
             (libwdi may install a one-time signing cert)"
                .into(),
        );
    }

    eprintln!("[flow8-ble-helper] listening on {PIPE_NAME}");
    let server = tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)
        .map_err(|e| format!("create named pipe: {e}"))?;

    eprintln!("[flow8-ble-helper] waiting for UI client…");
    server
        .connect()
        .await
        .map_err(|e| format!("pipe connect: {e}"))?;
    eprintln!("[flow8-ble-helper] client connected");

    let (reader, mut writer) = tokio::io::split(server);
    let mut lines = BufReader::new(reader).lines();

    let (out_tx, mut out_rx) = mpsc::channel::<ServerMsg>(64);
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            match encode_line(&msg) {
                Ok(line) => {
                    if writer.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                    if writer.flush().await.is_err() {
                        break;
                    }
                }
                Err(e) => eprintln!("[flow8-ble-helper] encode error: {e}"),
            }
        }
    });

    let send = |tx: &mpsc::Sender<ServerMsg>, msg: ServerMsg| {
        let tx = tx.clone();
        async move {
            let _ = tx.send(msg).await;
        }
    };

    send(&out_tx, ServerMsg::State {
        state: "Preparing".into(),
    })
    .await;

    let controller_id = match prepare::ensure_prepared(args.controller_id.as_deref()) {
        Ok(id) => {
            eprintln!("[flow8-ble-helper] controller ready: {id}");
            Some(id)
        }
        Err(e) => {
            send(
                &out_tx,
                ServerMsg::Error {
                    message: format!("prepare failed: {e}"),
                },
            )
            .await;
            let _ = writer_task.await;
            return Err(e);
        }
    };

    send(&out_tx, ServerMsg::Ready).await;

    let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(32);
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
    let mut write_rx = Some(write_rx);
    let mut cancel_rx = Some(cancel_rx);
    let mut session_task: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("pipe read: {e}"))?
    {
        if line.trim().is_empty() {
            continue;
        }
        let msg: ClientMsg = match decode_line(&line) {
            Ok(m) => m,
            Err(e) => {
                send(
                    &out_tx,
                    ServerMsg::Error {
                        message: format!("bad client message: {e}"),
                    },
                )
                .await;
                continue;
            }
        };

        match msg {
            ClientMsg::Connect => {
                if session_task.is_some() {
                    send(
                        &out_tx,
                        ServerMsg::Error {
                            message: "session already running".into(),
                        },
                    )
                    .await;
                    continue;
                }
                let Some(wrx) = write_rx.take() else {
                    send(
                        &out_tx,
                        ServerMsg::Error {
                            message: "write channel exhausted".into(),
                        },
                    )
                    .await;
                    continue;
                };
                let Some(crx) = cancel_rx.take() else {
                    send(
                        &out_tx,
                        ServerMsg::Error {
                            message: "cancel channel exhausted".into(),
                        },
                    )
                    .await;
                    continue;
                };
                let out = out_tx.clone();
                let ctrl = controller_id.clone();
                session_task = Some(tokio::spawn(async move {
                    if let Err(e) = run_session(ctrl, out.clone(), wrx, crx).await {
                        let _ = out
                            .send(ServerMsg::Error { message: e })
                            .await;
                        let _ = out
                            .send(ServerMsg::State {
                                state: "Disconnected".into(),
                            })
                            .await;
                    }
                }));
            }
            ClientMsg::Write { hex } => match hex_decode(&hex) {
                Ok(bytes) => {
                    if write_tx.send(bytes).await.is_err() {
                        send(
                            &out_tx,
                            ServerMsg::Error {
                                message: "session not accepting writes".into(),
                            },
                        )
                        .await;
                    }
                }
                Err(e) => {
                    send(
                        &out_tx,
                        ServerMsg::Error {
                            message: format!("bad write hex: {e}"),
                        },
                    )
                    .await;
                }
            },
            ClientMsg::Cancel => {
                let _ = cancel_tx.send(()).await;
                if let Some(handle) = session_task.take() {
                    let _ = handle.await;
                }
                send(
                    &out_tx,
                    ServerMsg::State {
                        state: "Disconnected".into(),
                    },
                )
                .await;
                eprintln!("[flow8-ble-helper] cancelled — exiting");
                drop(out_tx);
                let _ = writer_task.await;
                return Ok(());
            }
            ClientMsg::Shutdown => {
                let _ = cancel_tx.send(()).await;
                if let Some(handle) = session_task.take() {
                    let _ = handle.await;
                }
                eprintln!("[flow8-ble-helper] shutdown");
                drop(out_tx);
                let _ = writer_task.await;
                return Ok(());
            }
        }
    }

    let _ = cancel_tx.send(()).await;
    if let Some(handle) = session_task.take() {
        let _ = handle.await;
    }
    drop(out_tx);
    let _ = writer_task.await;
    eprintln!("[flow8-ble-helper] client disconnected");
    Ok(())
}

#[cfg(windows)]
async fn run_session(
    controller_id: Option<String>,
    out_tx: mpsc::Sender<ServerMsg>,
    write_rx: mpsc::Receiver<Vec<u8>>,
    cancel_rx: mpsc::Receiver<()>,
) -> Result<(), String> {
    let out_state = out_tx.clone();
    let out_notify = out_tx.clone();

    let on_state: session::StateFn = Box::new(move |state: &str| {
        let out = out_state.clone();
        let state = state.to_string();
        tokio::spawn(async move {
            let _ = out.send(ServerMsg::State { state }).await;
        });
    });

    let on_notify: session::NotifyFn = Box::new(move |bytes: &[u8]| {
        let out = out_notify.clone();
        let hex = hex_encode(bytes);
        tokio::spawn(async move {
            let _ = out.send(ServerMsg::Notify { hex }).await;
        });
    });

    let mut active = session::connect_and_handshake(controller_id, on_state, on_notify).await?;

    let out_notify2 = out_tx.clone();
    let on_notify_active: session::NotifyFn = Box::new(move |bytes: &[u8]| {
        let out = out_notify2.clone();
        let hex = hex_encode(bytes);
        tokio::spawn(async move {
            let _ = out.send(ServerMsg::Notify { hex }).await;
        });
    });

    let result =
        session::run_active_loop(&mut active, write_rx, on_notify_active, cancel_rx).await;
    session::shutdown_session(active).await;

    let _ = out_tx
        .send(ServerMsg::State {
            state: "Disconnected".into(),
        })
        .await;
    result
}

struct Args {
    trust: bool,
    controller_id: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut trust = false;
    let mut controller_id = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trust-certificate" => trust = true,
            "--controller" | "-c" => {
                controller_id = Some(args.next().ok_or("--controller requires an id")?);
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Args {
        trust,
        controller_id,
    })
}

fn print_help() {
    println!(
        "flow8-ble-helper — elevated FLOW 8 BLE session via DirectHCI\n\n\
         Usage:\n  \
           flow8-ble-helper --trust-certificate [--controller <id>]\n\n\
         Opens {PIPE_NAME} and waits for the flow-8-midi UI.\n\
         Requires Administrator."
    );
}
