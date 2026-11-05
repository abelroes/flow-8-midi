//! macOS/Linux BLE session via btleplug.

use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::log;
use crate::log_warn;
use crate::model::meter::{METER_REQUEST_INTERVAL, METER_TIMEOUT};
use crate::service::ble_bond;
use crate::service::ble_codec::encode_single;
use crate::service::ble_dispatcher::BleDispatcher;
use crate::service::ble_protocol::{cmd, BleCommand, BleEvent, DEFAULT_METER_IDS};
use crate::service::ble_session::{BleSessionHandle, SessionControl, SessionState};
use crate::service::client_id;

const SERVICE_UUID: &str = "14839ad4-8d7e-415c-9a42-167340cf2339";
const CHARACTERISTIC_UUID: &str = "0034594a-a8e7-4b1a-a6b1-cd5243059a57";
const FLOW8_NAME_KEYWORD: &str = "FLOW";
const SCAN_TIMEOUT: Duration = Duration::from_secs(12);
const RESCAN_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const PAIRING_TIMEOUT: Duration = Duration::from_secs(30);
const BOND_SETTLE: Duration = Duration::from_secs(2);
const GATT_SETTLE: Duration = Duration::from_millis(500);
/// One automatic full-session retry after a recoverable BlueZ failure.
const MAX_SESSION_AUTO_RETRIES: u32 = 1;

fn service_uuid() -> Uuid {
    Uuid::parse_str(SERVICE_UUID).unwrap()
}

fn characteristic_uuid() -> Uuid {
    Uuid::parse_str(CHARACTERISTIC_UUID).unwrap()
}

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
    let client_id = client_id::load_or_create();
    let _ = state_tx.send(SessionState::Disconnected);

    loop {
        if !wait_for_connect(&mut control_rx).await {
            return;
        }

        let mut auto_retries = 0u32;
        loop {
            if matches!(control_rx.try_recv(), Ok(SessionControl::Cancel)) {
                let _ = state_tx.send(SessionState::Disconnected);
                break;
            }

            let result = run_session(
                &client_id,
                &event_tx,
                &mut command_rx,
                &mut control_rx,
                &state_tx,
            )
            .await;

            match result {
                Ok(()) => {
                    let _ = state_tx.send(SessionState::Disconnected);
                    let _ = event_tx
                        .send(BleEvent::Disconnected("Session ended".to_string()))
                        .await;
                    break;
                }
                Err(e)
                    if auto_retries < MAX_SESSION_AUTO_RETRIES
                        && ble_bond::is_recoverable_session_error(&e) =>
                {
                    auto_retries += 1;
                    log_warn!(
                        "[BLE_SESSION] Recoverable BlueZ failure — auto-retry {}/{}: {}",
                        auto_retries,
                        MAX_SESSION_AUTO_RETRIES,
                        e
                    );
                    tokio::time::sleep(BOND_SETTLE).await;
                }
                Err(e) => {
                    eprintln!("[BLE_SESSION] Error: {}", e);
                    let _ = state_tx.send(SessionState::Disconnected);
                    let _ = event_tx.send(BleEvent::Disconnected(e)).await;
                    break;
                }
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

async fn run_session(
    client_id: &[u8; 16],
    event_tx: &mpsc::Sender<BleEvent>,
    command_rx: &mut mpsc::Receiver<BleCommand>,
    control_rx: &mut mpsc::Receiver<SessionControl>,
    state_tx: &tokio::sync::watch::Sender<SessionState>,
) -> Result<(), String> {
    let manager = Manager::new()
        .await
        .map_err(|e| format!("BLE manager: {}", e))?;

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("BLE adapters: {}", e))?;

    let adapter = adapters.into_iter().next().ok_or("No BLE adapter found")?;

    let _ = state_tx.send(SessionState::Scanning);

    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| format!("Scan start: {}", e))?;

    tokio::select! {
        _ = tokio::time::sleep(SCAN_TIMEOUT) => {}
        ctrl = control_rx.recv() => {
            adapter.stop_scan().await.ok();
            match ctrl {
                Some(SessionControl::Cancel) | None => {
                    return Err("Connection cancelled".to_string());
                }
                Some(SessionControl::Connect) => {}
            }
        }
    }
    adapter.stop_scan().await.ok();

    if matches!(control_rx.try_recv(), Ok(SessionControl::Cancel)) {
        return Err("Connection cancelled".to_string());
    }

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripheral list: {}", e))?;

    let mut peripheral = find_flow8(&peripherals).await?;
    let address = peripheral_address(&peripheral).await;
    log!("[BLE_SESSION] Device address 0x{:012X}", address & 0xFFFF_FFFF_FFFF);

    let _ = state_tx.send(SessionState::Connecting);
    let characteristic =
        open_flow8_gatt(&adapter, &mut peripheral, address, control_rx, state_tx).await?;

    let mut notification_stream = peripheral
        .notifications()
        .await
        .map_err(|e| format!("Notification stream: {}", e))?;

    let _ = state_tx.send(SessionState::WaitingHandshake);

    let handshake_host = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        let mut dispatcher = BleDispatcher::new();
        loop {
            if let Some(notification) = notification_stream.next().await {
                if let Ok(Some(event)) = dispatcher.dispatch(&notification.value) {
                    if let BleEvent::HandshakeHost(data) = event {
                        return Ok(data);
                    }
                }
            }
        }
    })
    .await
    .map_err(|_| "Timeout waiting for HandshakeHost (0x35)")?;

    let handshake_data =
        handshake_host.map_err(|e: String| format!("HandshakeHost parse: {}", e))?;

    let _ = event_tx
        .send(BleEvent::HandshakeHost(handshake_data.clone()))
        .await;

    if !handshake_data.pairing_any {
        let _ = state_tx.send(SessionState::Authenticating);

        let handshake_pkt =
            crate::service::ble_protocol::encode_command(&BleCommand::HandshakeClient {
                client_id: *client_id,
            })
            .map_err(|e| format!("Encode handshake: {}", e))?;

        write_characteristic(&peripheral, &characteristic, &handshake_pkt).await?;

        let got_ack = tokio::time::timeout(Duration::from_secs(5), async {
            let mut dispatcher = BleDispatcher::new();
            loop {
                if let Some(notification) = notification_stream.next().await {
                    if let Ok(Some(event)) = dispatcher.dispatch(&notification.value) {
                        match event {
                            BleEvent::HandshakeAck => return true,
                            BleEvent::HandshakeHost(new_data) => {
                                if new_data.pairing_any {
                                    return false;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        })
        .await;

        match got_ack {
            Ok(true) => {}
            _ => {
                let _ = state_tx.send(SessionState::PairingRequired);
                let _ = event_tx.send(BleEvent::PairingRequired).await;

                let pairing_result = tokio::time::timeout(
                    PAIRING_TIMEOUT,
                    wait_for_pairing(&mut notification_stream),
                )
                .await;

                match pairing_result {
                    Ok(Ok(())) => {
                        write_characteristic(&peripheral, &characteristic, &handshake_pkt)
                            .await?;
                        tokio::time::timeout(Duration::from_secs(5), async {
                            let mut dispatcher = BleDispatcher::new();
                            loop {
                                if let Some(notification) = notification_stream.next().await {
                                    if let Ok(Some(BleEvent::HandshakeAck)) =
                                        dispatcher.dispatch(&notification.value)
                                    {
                                        return;
                                    }
                                }
                            }
                        })
                        .await
                        .map_err(|_| "Timeout waiting for ack after pairing")?;
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        return Err(
                            "Pairing timeout (30s) — press the pairing button on the mixer"
                                .to_string(),
                        );
                    }
                }
            }
        }
    } else {
        let _ = state_tx.send(SessionState::Authenticating);

        let handshake_pkt =
            crate::service::ble_protocol::encode_command(&BleCommand::HandshakeClient {
                client_id: *client_id,
            })
            .map_err(|e| format!("Encode handshake: {}", e))?;

        write_characteristic(&peripheral, &characteristic, &handshake_pkt).await?;

        tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            let mut dispatcher = BleDispatcher::new();
            loop {
                if let Some(notification) = notification_stream.next().await {
                    if let Ok(Some(BleEvent::HandshakeAck)) =
                        dispatcher.dispatch(&notification.value)
                    {
                        return;
                    }
                }
            }
        })
        .await
        .map_err(|_| "Timeout waiting for HandshakeReply (0x36)")?;
    }

    let _ = event_tx.send(BleEvent::HandshakeAck).await;

    let ack_pkt = encode_single(cmd::HANDSHAKE_REPLY, &[]);
    write_characteristic(&peripheral, &characteristic, &ack_pkt).await?;

    let _ = state_tx.send(SessionState::RequestingState);

    let get_state_pkt = crate::service::ble_protocol::encode_command(&BleCommand::GetMixerState)
        .map_err(|e| format!("Encode GetMixerState: {}", e))?;
    write_characteristic(&peripheral, &characteristic, &get_state_pkt).await?;

    let mixer_state_result = tokio::time::timeout(Duration::from_secs(10), async {
        let mut dispatcher = BleDispatcher::new();
        loop {
            if let Some(notification) = notification_stream.next().await {
                if let Ok(Some(event)) = dispatcher.dispatch(&notification.value) {
                    if let BleEvent::MixerState(payload) = event {
                        return Ok(payload);
                    }
                }
            }
        }
    })
    .await
    .map_err(|_| "Timeout waiting for MixerState (0x38)")?;

    let state_payload: Vec<u8> =
        mixer_state_result.map_err(|e: String| format!("MixerState: {}", e))?;
    let _ = event_tx.send(BleEvent::MixerState(state_payload)).await;

    let _ = state_tx.send(SessionState::Active);

    let mut dispatcher = BleDispatcher::new();
    let mut meter_tick = tokio::time::interval(METER_REQUEST_INTERVAL);
    meter_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_meter_update: Option<tokio::time::Instant> = None;
    let mut meter_timeout_logged = false;

    loop {
        tokio::select! {
            Some(notification) = notification_stream.next() => {
                match dispatcher.dispatch(&notification.value) {
                    Ok(Some(event)) => {
                        if matches!(event, BleEvent::MeterUpdate(_)) {
                            last_meter_update = Some(tokio::time::Instant::now());
                            meter_timeout_logged = false;
                        }
                        if event_tx.send(event).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("[BLE_SESSION] Dispatch error: {}", e);
                    }
                }
            }
            Some(command) = command_rx.recv() => {
                match crate::service::ble_protocol::encode_command(&command) {
                    Ok(pkt) => {
                        if let Err(e) = write_characteristic(&peripheral, &characteristic, &pkt).await {
                            eprintln!("[BLE_SESSION] Write error: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[BLE_SESSION] Encode error: {}", e);
                    }
                }
            }
            ctrl = control_rx.recv() => {
                match ctrl {
                    Some(SessionControl::Cancel) | None => {
                        peripheral.disconnect().await.ok();
                        return Err("Connection cancelled".to_string());
                    }
                    Some(SessionControl::Connect) => {}
                }
            }
            _ = meter_tick.tick() => {
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
                        if let Err(e) =
                            write_characteristic(&peripheral, &characteristic, &pkt).await
                        {
                            eprintln!("[BLE_SESSION] Meter request error: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[BLE_SESSION] Meter encode error: {}", e);
                    }
                }
            }
            else => break,
        }
    }

    Ok(())
}

fn is_cccd_denied(err: &impl ToString) -> bool {
    let s = err.to_string().to_lowercase();
    s.contains("80650003")
        || s.contains("cannot be written")
        || s.contains("write not permitted")
        || s.contains("protocolerror")
}

async fn peripheral_address(peripheral: &Peripheral) -> u64 {
    peripheral
        .properties()
        .await
        .ok()
        .flatten()
        .map(|p| u64::from(p.address))
        .unwrap_or_else(|| u64::from(peripheral.address()))
}

async fn find_flow8(peripherals: &[Peripheral]) -> Result<Peripheral, String> {
    let target_svc = service_uuid();

    for p in peripherals {
        if let Ok(Some(props)) = p.properties().await {
            if let Some(ref name) = props.local_name {
                if name.to_uppercase().contains(FLOW8_NAME_KEYWORD) {
                    return Ok(p.clone());
                }
            }
            if props.services.contains(&target_svc) {
                return Ok(p.clone());
            }
        }
    }

    Err(
        "FLOW 8 LE not found during scan. Make sure the mixer is powered on and Bluetooth is enabled on this computer."
            .to_string(),
    )
}

/// Linear BlueZ link open with at most one bond-clear / rediscover step.
///
/// 1. Connect + discover + subscribe  
/// 2. On `key-missing` → RemoveDevice → settle → rescan → connect once  
/// 3. On stale Device1 → rescan → connect once  
/// 4. On CCCD denied → disconnect → settle → connect + subscribe once  
async fn open_flow8_gatt(
    adapter: &btleplug::platform::Adapter,
    peripheral: &mut Peripheral,
    address: u64,
    control_rx: &mut mpsc::Receiver<SessionControl>,
    state_tx: &tokio::sync::watch::Sender<SessionState>,
) -> Result<Characteristic, String> {
    let mut characteristic = match connect_once(peripheral).await {
        Ok(c) => c,
        Err(e) if ble_bond::is_link_key_missing(&e) => {
            log_warn!("[BLE_SESSION] Link key missing — clear bond once, then reconnect");
            ble_bond::force_rebond(address).await.map_err(|rebond_err| {
                format!("{}. {}. USB MIDI remains available.", e, rebond_err)
            })?;
            peripheral.disconnect().await.ok();
            tokio::time::sleep(BOND_SETTLE).await;
            let _ = state_tx.send(SessionState::Scanning);
            *peripheral = scan_for_flow8(adapter, control_rx, RESCAN_TIMEOUT).await?;
            log!(
                "[BLE_SESSION] Rediscovered 0x{:012X}",
                peripheral_address(peripheral).await & 0xFFFF_FFFF_FFFF
            );
            let _ = state_tx.send(SessionState::Connecting);
            match connect_once(peripheral).await {
                Ok(c) => c,
                Err(e2) if ble_bond::is_stale_device_error(&e2) => {
                    log_warn!("[BLE_SESSION] Stale after bond clear — rescan once more");
                    peripheral.disconnect().await.ok();
                    tokio::time::sleep(GATT_SETTLE).await;
                    let _ = state_tx.send(SessionState::Scanning);
                    *peripheral = scan_for_flow8(adapter, control_rx, RESCAN_TIMEOUT).await?;
                    let _ = state_tx.send(SessionState::Connecting);
                    connect_once(peripheral).await.map_err(|e3| {
                        format!(
                            "{}. Bond cleared once. If this persists, remove FLOW LE in system Bluetooth settings. USB MIDI remains available.",
                            e3
                        )
                    })?
                }
                Err(e2) => {
                    return Err(format!(
                        "{}. Bond cleared once. If this persists, remove FLOW LE in system Bluetooth settings. USB MIDI remains available.",
                        e2
                    ));
                }
            }
        }
        Err(e) if ble_bond::is_stale_device_error(&e) => {
            log_warn!("[BLE_SESSION] Stale BlueZ device — rescan once");
            peripheral.disconnect().await.ok();
            tokio::time::sleep(GATT_SETTLE).await;
            let _ = state_tx.send(SessionState::Scanning);
            *peripheral = scan_for_flow8(adapter, control_rx, RESCAN_TIMEOUT).await?;
            let _ = state_tx.send(SessionState::Connecting);
            connect_once(peripheral).await?
        }
        Err(e) => return Err(e),
    };

    let _ = state_tx.send(SessionState::Subscribing);
    match peripheral.subscribe(&characteristic).await {
        Ok(()) => {
            log!("[BLE_SESSION] Subscribe OK");
            Ok(characteristic)
        }
        Err(e) if is_cccd_denied(&e) => {
            log_warn!("[BLE_SESSION] CCCD denied — one clean reconnect");
            peripheral.disconnect().await.ok();
            tokio::time::sleep(BOND_SETTLE).await;
            let _ = state_tx.send(SessionState::Connecting);
            characteristic = connect_once(peripheral).await?;
            let _ = state_tx.send(SessionState::Subscribing);
            peripheral.subscribe(&characteristic).await.map_err(|e2| {
                format!(
                    "Subscribe failed after reconnect: {}. USB MIDI remains available.",
                    e2
                )
            })?;
            log!("[BLE_SESSION] Subscribe OK after reconnect");
            Ok(characteristic)
        }
        Err(e) => {
            peripheral.disconnect().await.ok();
            Err(format!("Subscribe failed: {}", e))
        }
    }
}

async fn scan_for_flow8(
    adapter: &btleplug::platform::Adapter,
    control_rx: &mut mpsc::Receiver<SessionControl>,
    timeout: Duration,
) -> Result<Peripheral, String> {
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| format!("Scan start: {}", e))?;

    tokio::select! {
        _ = tokio::time::sleep(timeout) => {}
        ctrl = control_rx.recv() => {
            adapter.stop_scan().await.ok();
            match ctrl {
                Some(SessionControl::Cancel) | None => {
                    return Err("Connection cancelled".to_string());
                }
                Some(SessionControl::Connect) => {}
            }
        }
    }
    adapter.stop_scan().await.ok();

    if matches!(control_rx.try_recv(), Ok(SessionControl::Cancel)) {
        return Err("Connection cancelled".to_string());
    }

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripheral list: {}", e))?;
    find_flow8(&peripherals).await
}

/// Single connect attempt — no retry loop. Bond/stale recovery happens one level up.
async fn connect_once(peripheral: &Peripheral) -> Result<Characteristic, String> {
    match tokio::time::timeout(CONNECT_TIMEOUT, peripheral.connect()).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            let err = format!("Connect: {}", e);
            peripheral.disconnect().await.ok();
            return Err(err);
        }
        Err(_) => {
            peripheral.disconnect().await.ok();
            return Err("Connection timed out".to_string());
        }
    }

    if let Err(e) = peripheral.discover_services().await {
        peripheral.disconnect().await.ok();
        return Err(format!("Service discovery: {}", e));
    }

    let target = characteristic_uuid();
    match peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == target)
    {
        Some(c) => Ok(c),
        None => {
            peripheral.disconnect().await.ok();
            Err("BLE characteristic not found".to_string())
        }
    }
}

/// One rewrite on transient ATT errors (0x0E), then fail.
async fn write_characteristic(
    peripheral: &Peripheral,
    characteristic: &Characteristic,
    data: &[u8],
) -> Result<(), String> {
    match peripheral
        .write(characteristic, data, WriteType::WithResponse)
        .await
    {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = format!("BLE write: {}", e);
            if !ble_bond::is_transient_gatt_error(&msg) {
                return Err(msg);
            }
            log_warn!("[BLE_SESSION] Transient GATT write — one rewrite ({})", msg);
            tokio::time::sleep(GATT_SETTLE).await;
            peripheral
                .write(characteristic, data, WriteType::WithResponse)
                .await
                .map_err(|e2| format!("BLE write: {}", e2))
        }
    }
}

async fn wait_for_pairing(
    stream: &mut (impl StreamExt<Item = btleplug::api::ValueNotification> + Unpin),
) -> Result<(), String> {
    let mut dispatcher = BleDispatcher::new();
    loop {
        if let Some(notification) = stream.next().await {
            if let Ok(Some(event)) = dispatcher.dispatch(&notification.value) {
                if let BleEvent::HandshakeHost(data) = event {
                    if data.pairing_any {
                        return Ok(());
                    }
                }
            }
        }
    }
}
