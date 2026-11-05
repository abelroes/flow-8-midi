//! Windows BLE session via elevated `flow8-ble-helper` (DirectHCI).

use std::time::Duration;

use tokio::sync::mpsc;

use crate::log;
use crate::model::meter::{METER_REQUEST_INTERVAL, METER_TIMEOUT};
use crate::service::ble_dispatcher::BleDispatcher;
use crate::service::ble_helper_ipc::{self, ClientMsg, HelperPipe, ServerMsg};
use crate::service::ble_protocol::{BleCommand, BleEvent, DEFAULT_METER_IDS};
use crate::service::ble_session::{BleSessionHandle, SessionControl, SessionState};

pub fn start_session() -> BleSessionHandle {
    let (event_tx, event_rx) = mpsc::channel::<BleEvent>(64);
    let (command_tx, command_rx) = mpsc::channel::<BleCommand>(64);
    let (control_tx, control_rx) = mpsc::channel::<SessionControl>(8);
    let (state_tx, state_rx) = tokio::sync::watch::channel(SessionState::Disconnected);

    tokio::spawn(async move {
        session_loop(event_tx, command_rx, control_rx, state_tx).await;
    });

    BleSessionHandle {
        command_tx,
        control_tx,
        event_rx,
        state_rx,
    }
}

async fn session_loop(
    event_tx: mpsc::Sender<BleEvent>,
    mut command_rx: mpsc::Receiver<BleCommand>,
    mut control_rx: mpsc::Receiver<SessionControl>,
    state_tx: tokio::sync::watch::Sender<SessionState>,
) {
    let _ = state_tx.send(SessionState::Disconnected);

    loop {
        if !wait_for_connect(&mut control_rx).await {
            return;
        }

        match run_once(&event_tx, &mut command_rx, &mut control_rx, &state_tx).await {
            Ok(()) => {
                let _ = state_tx.send(SessionState::Disconnected);
                let _ = event_tx
                    .send(BleEvent::Disconnected("Session ended".into()))
                    .await;
            }
            Err(e) => {
                eprintln!("[BLE_SESSION] Error: {e}");
                let _ = state_tx.send(SessionState::Disconnected);
                let _ = event_tx.send(BleEvent::Disconnected(e)).await;
            }
        }
    }
}

async fn wait_for_connect(control_rx: &mut mpsc::Receiver<SessionControl>) -> bool {
    loop {
        match control_rx.recv().await {
            Some(SessionControl::Connect) => return true,
            Some(SessionControl::Cancel) => {}
            None => return false,
        }
    }
}

async fn run_once(
    event_tx: &mpsc::Sender<BleEvent>,
    command_rx: &mut mpsc::Receiver<BleCommand>,
    control_rx: &mut mpsc::Receiver<SessionControl>,
    state_tx: &tokio::sync::watch::Sender<SessionState>,
) -> Result<(), String> {
    let _ = state_tx.send(SessionState::Preparing);
    log!("[BLE_SESSION] Launching elevated DirectHCI helper…");

    ble_helper_ipc::launch_elevated()?;

    let mut pipe = HelperPipe::connect_with_retry(Duration::from_secs(120)).await?;

    // Drain until Ready (Preparing / errors along the way).
    loop {
        tokio::select! {
            ctrl = control_rx.recv() => {
                match ctrl {
                    Some(SessionControl::Cancel) | None => {
                        let _ = pipe.send(&ClientMsg::Cancel).await;
                        return Err("Connection cancelled".into());
                    }
                    Some(SessionControl::Connect) => {}
                }
            }
            msg = pipe.recv() => {
                match msg? {
                    Some(ServerMsg::Ready) => break,
                    Some(ServerMsg::State { state }) => {
                        if let Some(s) = parse_state(&state) {
                            let _ = state_tx.send(s);
                        }
                    }
                    Some(ServerMsg::Error { message }) => {
                        return Err(message);
                    }
                    Some(ServerMsg::Notify { .. }) => {}
                    None => return Err("helper pipe closed before Ready".into()),
                }
            }
        }
    }

    pipe.send(&ClientMsg::Connect).await?;

    let (out_tx, mut out_rx) = mpsc::channel::<ClientMsg>(32);
    let (raw_tx, raw_rx) = mpsc::channel::<Vec<u8>>(32);

    // Encode BleCommands → raw bytes → ClientMsg::Write
    let encode_tx = out_tx.clone();
    tokio::spawn(async move {
        ble_helper_ipc::write_forward_loop(raw_rx, encode_tx).await;
    });

    let mut dispatcher = BleDispatcher::new();
    let mut meter_tick = tokio::time::interval(METER_REQUEST_INTERVAL);
    meter_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_meter_update: Option<tokio::time::Instant> = None;
    let mut meter_timeout_logged = false;
    let mut active = false;

    loop {
        tokio::select! {
            biased;
            ctrl = control_rx.recv() => {
                match ctrl {
                    Some(SessionControl::Cancel) | None => {
                        let _ = pipe.send(&ClientMsg::Cancel).await;
                        return Err("Connection cancelled".into());
                    }
                    Some(SessionControl::Connect) => {}
                }
            }
            Some(out) = out_rx.recv() => {
                pipe.send(&out).await?;
            }
            msg = pipe.recv() => {
                match msg? {
                    None => return Err("helper disconnected".into()),
                    Some(ServerMsg::Ready) => {}
                    Some(ServerMsg::State { state }) => {
                        if let Some(s) = parse_state(&state) {
                            if s == SessionState::Active {
                                active = true;
                            }
                            if s == SessionState::Disconnected && active {
                                return Ok(());
                            }
                            let _ = state_tx.send(s);
                        }
                    }
                    Some(ServerMsg::Error { message }) => {
                        return Err(message);
                    }
                    Some(ServerMsg::Notify { hex }) => {
                        let value = ble_helper_ipc::hex_decode(&hex)?;
                        match dispatcher.dispatch(&value) {
                            Ok(Some(event)) => {
                                if matches!(event, BleEvent::MeterUpdate(_)) {
                                    last_meter_update = Some(tokio::time::Instant::now());
                                    meter_timeout_logged = false;
                                }
                                if matches!(event, BleEvent::MixerState(_)) && !active {
                                    // Helper reaches Active after first MixerState; keep UI in sync.
                                }
                                if event_tx.send(event).await.is_err() {
                                    let _ = pipe.send(&ClientMsg::Shutdown).await;
                                    return Ok(());
                                }
                            }
                            Ok(None) => {}
                            Err(e) => eprintln!("[BLE_SESSION] Dispatch error: {e}"),
                        }
                    }
                }
            }
            Some(command) = command_rx.recv(), if active => {
                match crate::service::ble_protocol::encode_command(&command) {
                    Ok(pkt) => {
                        if raw_tx.send(pkt).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => eprintln!("[BLE_SESSION] Encode error: {e}"),
                }
            }
            _ = meter_tick.tick(), if active => {
                if let Some(last) = last_meter_update {
                    if last.elapsed() > METER_TIMEOUT && !meter_timeout_logged {
                        eprintln!("[BLE_SESSION] Meter updates stalled (> {:?})", METER_TIMEOUT);
                        meter_timeout_logged = true;
                    }
                }
                let meter_cmd = BleCommand::MeterRequest {
                    channel_ids: DEFAULT_METER_IDS.to_vec(),
                };
                match crate::service::ble_protocol::encode_command(&meter_cmd) {
                    Ok(pkt) => {
                        if raw_tx.send(pkt).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => eprintln!("[BLE_SESSION] Meter encode error: {e}"),
                }
            }
        }
    }

    let _ = pipe.send(&ClientMsg::Shutdown).await;
    Ok(())
}

fn parse_state(s: &str) -> Option<SessionState> {
    match s {
        "Disconnected" => Some(SessionState::Disconnected),
        "Preparing" => Some(SessionState::Preparing),
        "Scanning" => Some(SessionState::Scanning),
        "Connecting" => Some(SessionState::Connecting),
        "Subscribing" => Some(SessionState::Subscribing),
        "WaitingHandshake" => Some(SessionState::WaitingHandshake),
        "PairingRequired" => Some(SessionState::PairingRequired),
        "Authenticating" => Some(SessionState::Authenticating),
        "RequestingState" => Some(SessionState::RequestingState),
        "Active" => Some(SessionState::Active),
        _ => None,
    }
}
