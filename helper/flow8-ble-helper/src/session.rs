//! FLOW 8 BLE session over DirectHCI (passive listen, framed HandshakeClient).

use std::str::FromStr;
use std::time::Duration;

use directhci_ble::{
    BleAddress, BleCentralConfig, BleConnection, BleUuid, DirectHciBleCentral, GattCharacteristic,
    GattNotificationStream, SubscriptionMode, WriteMode,
};
use tokio::sync::mpsc;

use crate::wire::{
    encode_single, load_or_create_client_id, parse_handshake_host, CMD_GET_MIXER_STATE,
    CMD_HANDSHAKE_CLIENT, CMD_HANDSHAKE_REPLY, CMD_MIXER_STATE,
};

const SERVICE_UUID: &str = "14839ad4-8d7e-415c-9a42-167340cf2339";
const CHARACTERISTIC_UUID: &str = "0034594a-a8e7-4b1a-a6b1-cd5243059a57";
const SCAN_SECS: u64 = 8;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const PAIRING_TIMEOUT: Duration = Duration::from_secs(30);
const ACK_TIMEOUT: Duration = Duration::from_secs(5);

pub type StateFn = Box<dyn FnMut(&str) + Send>;
pub type NotifyFn = Box<dyn FnMut(&[u8]) + Send>;

pub struct ActiveSession {
    pub connection: BleConnection,
    pub characteristic: GattCharacteristic,
    pub notifications: GattNotificationStream,
}

pub async fn connect_and_handshake(
    controller_id: Option<String>,
    mut on_state: StateFn,
    mut on_notify: NotifyFn,
) -> Result<ActiveSession, String> {
    on_state("Connecting");
    let client_id = load_or_create_client_id();
    let central = DirectHciBleCentral::connect(BleCentralConfig {
        controller_id,
        client_name: "flow8-ble-helper".into(),
        client_version: Some(env!("CARGO_PKG_VERSION").into()),
    })
    .await
    .map_err(|e| format!("acquire DirectHCI: {e}"))?;

    on_state("Scanning");
    let peer = match scan_for_flow8(&central).await {
        Ok(addr) => addr,
        Err(e) => {
            let _ = central.shutdown().await;
            return Err(e);
        }
    };

    on_state("Connecting");
    let connection = match central.connect_device(peer).await {
        Ok(c) => c,
        Err(e) => return Err(format!("connect: {e}")),
    };

    let services = match connection.discover().await {
        Ok(s) => s,
        Err(e) => {
            let _ = connection.disconnect().await;
            return Err(format!("gatt-discover: {e}"));
        }
    };

    let char_uuid = BleUuid::from_str(CHARACTERISTIC_UUID).map_err(|e| e.to_string())?;
    let characteristic = services
        .iter()
        .flat_map(|s| s.characteristics.iter())
        .find(|c| c.uuid == char_uuid)
        .cloned()
        .ok_or_else(|| format!("characteristic {CHARACTERISTIC_UUID} missing"))?;

    on_state("Subscribing");
    let mut stream = match enable_notifications(&connection, &characteristic).await {
        Ok(s) => s,
        Err(e) => {
            let _ = connection.disconnect().await;
            return Err(e);
        }
    };

    if let Err(e) =
        handshake_and_state(&connection, &characteristic, &mut stream, &client_id, &mut on_state, &mut on_notify)
            .await
    {
        let _ = stream.close().await;
        let _ = connection.disconnect().await;
        return Err(e);
    }

    on_state("Active");
    Ok(ActiveSession {
        connection,
        characteristic,
        notifications: stream,
    })
}

pub async fn run_active_loop(
    session: &mut ActiveSession,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut on_notify: NotifyFn,
    mut cancel_rx: mpsc::Receiver<()>,
) -> Result<(), String> {
    loop {
        tokio::select! {
            biased;
            _ = cancel_rx.recv() => {
                return Ok(());
            }
            Some(pkt) = write_rx.recv() => {
                session
                    .connection
                    .write(&session.characteristic, pkt, WriteMode::WithResponse)
                    .await
                    .map_err(|e| format!("write: {e}"))?;
            }
            event = session.notifications.recv() => {
                match event {
                    Ok(Some(ev)) => {
                        if !ev.value.is_empty() {
                            on_notify(&ev.value);
                        }
                    }
                    Ok(None) => return Err("notification stream closed".into()),
                    Err(e) => return Err(format!("notify: {e}")),
                }
            }
        }
    }
}

pub async fn shutdown_session(session: ActiveSession) {
    let _ = session.notifications.close().await;
    let _ = session.connection.disconnect().await;
}

async fn scan_for_flow8(central: &DirectHciBleCentral) -> Result<BleAddress, String> {
    let results = central
        .scan(Duration::from_secs(SCAN_SECS))
        .await
        .map_err(|e| format!("scan: {e}"))?;
    let service = BleUuid::from_str(SERVICE_UUID).map_err(|e| e.to_string())?;
    for r in &results {
        let name = r.local_name.as_deref().unwrap_or("");
        let name_hit = name.to_ascii_uppercase().contains("FLOW");
        let uuid_hit = r.service_uuids.iter().any(|u| u == &service);
        if name_hit || uuid_hit {
            return Ok(r.address);
        }
    }
    Err(format!(
        "FLOW LE not found after {SCAN_SECS}s ({} ads)",
        results.len()
    ))
}

async fn enable_notifications(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
) -> Result<GattNotificationStream, String> {
    const CCCD_NOTIFY_ENABLE: [u8; 2] = [0x01, 0x00];

    if let Some(_cccd) = characteristic.cccd_handle {
        return connection
            .subscribe(characteristic, SubscriptionMode::Notifications)
            .await
            .map_err(|e| format!("subscribe CCCD: {e}"));
    }

    let guessed = characteristic.value_handle.saturating_add(1);
    let _ = connection
        .write_handle(guessed, CCCD_NOTIFY_ENABLE, WriteMode::WithResponse)
        .await;

    connection
        .listen(characteristic)
        .await
        .map_err(|e| format!("passive-listen: {e}"))
}

async fn handshake_and_state(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
    stream: &mut GattNotificationStream,
    client_id: &[u8; 16],
    on_state: &mut StateFn,
    on_notify: &mut NotifyFn,
) -> Result<(), String> {
    on_state("WaitingHandshake");
    let host = tokio::time::timeout(HANDSHAKE_TIMEOUT, wait_handshake_host(stream, on_notify))
        .await
        .map_err(|_| "timeout waiting HandshakeHost (0x35)".to_string())??;

    let handshake_client = encode_single(CMD_HANDSHAKE_CLIENT, client_id);
    on_state("Authenticating");
    connection
        .write(
            characteristic,
            handshake_client.clone(),
            WriteMode::WithResponse,
        )
        .await
        .map_err(|e| format!("write-0x39: {e}"))?;

    if !host.pairing_any {
        match ensure_handshake_ack(stream, on_notify, on_state).await? {
            AckOutcome::Ack | AckOutcome::PairingAnyNow => {}
            AckOutcome::NeedResend => {
                connection
                    .write(
                        characteristic,
                        handshake_client.clone(),
                        WriteMode::WithResponse,
                    )
                    .await
                    .map_err(|e| format!("write-0x39-resend: {e}"))?;
                tokio::time::timeout(ACK_TIMEOUT, wait_cmd(stream, CMD_HANDSHAKE_REPLY, on_notify))
                    .await
                    .map_err(|_| "timeout waiting 0x36 after pairing".to_string())??;
            }
        }
    } else {
        wait_handshake_reply_or_retry(connection, characteristic, stream, &handshake_client, on_notify)
            .await?;
    }

    on_state("RequestingState");
    let ack = encode_single(CMD_HANDSHAKE_REPLY, &[]);
    let get_state = encode_single(CMD_GET_MIXER_STATE, &[]);
    connection
        .write(characteristic, ack, WriteMode::WithResponse)
        .await
        .map_err(|e| format!("write-0x36-ack: {e}"))?;
    connection
        .write(characteristic, get_state, WriteMode::WithResponse)
        .await
        .map_err(|e| format!("write-0x37: {e}"))?;

    let first = tokio::time::timeout(HANDSHAKE_TIMEOUT, wait_cmd(stream, CMD_MIXER_STATE, on_notify))
        .await
        .map_err(|_| "timeout waiting MixerState (0x38)".to_string())??;
    on_notify(&first);
    Ok(())
}

enum AckOutcome {
    Ack,
    PairingAnyNow,
    NeedResend,
}

async fn ensure_handshake_ack(
    stream: &mut GattNotificationStream,
    on_notify: &mut NotifyFn,
    on_state: &mut StateFn,
) -> Result<AckOutcome, String> {
    let first = tokio::time::timeout(ACK_TIMEOUT, wait_ack_or_rehost(stream, on_notify)).await;
    match first {
        Ok(Ok(AckOutcome::Ack)) => return Ok(AckOutcome::Ack),
        Ok(Ok(AckOutcome::PairingAnyNow)) => {
            tokio::time::timeout(ACK_TIMEOUT, wait_cmd(stream, CMD_HANDSHAKE_REPLY, on_notify))
                .await
                .map_err(|_| "timeout 0x36 after pairing_any".to_string())??;
            return Ok(AckOutcome::Ack);
        }
        Ok(Ok(AckOutcome::NeedResend)) => unreachable!(),
        Ok(Err(e)) => return Err(e),
        Err(_) => {}
    }

    on_state("PairingRequired");
    tokio::time::timeout(PAIRING_TIMEOUT, wait_pairing_ready(stream, on_notify))
        .await
        .map_err(|_| "pairing timeout (30s) — press PAIR APP on the mixer".to_string())??;
    Ok(AckOutcome::NeedResend)
}

async fn wait_ack_or_rehost(
    stream: &mut GattNotificationStream,
    on_notify: &mut NotifyFn,
) -> Result<AckOutcome, String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        on_notify(&event.value);
        match event.value[0] {
            CMD_HANDSHAKE_REPLY => return Ok(AckOutcome::Ack),
            crate::wire::CMD_HANDSHAKE_HOST => {
                let host = parse_handshake_host(&event.value)?;
                if host.pairing_any {
                    return Ok(AckOutcome::PairingAnyNow);
                }
            }
            _ => {}
        }
    }
}

async fn wait_pairing_ready(
    stream: &mut GattNotificationStream,
    on_notify: &mut NotifyFn,
) -> Result<(), String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        on_notify(&event.value);
        if event.value[0] == crate::wire::CMD_HANDSHAKE_HOST {
            let host = parse_handshake_host(&event.value)?;
            if host.pairing_any {
                return Ok(());
            }
        }
        if event.value[0] == CMD_HANDSHAKE_REPLY {
            return Ok(());
        }
    }
}

async fn wait_handshake_host(
    stream: &mut GattNotificationStream,
    on_notify: &mut NotifyFn,
) -> Result<crate::wire::HandshakeHost, String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        on_notify(&event.value);
        if event.value[0] == crate::wire::CMD_HANDSHAKE_HOST {
            return parse_handshake_host(&event.value);
        }
    }
}

async fn wait_cmd(
    stream: &mut GattNotificationStream,
    cmd: u8,
    on_notify: &mut NotifyFn,
) -> Result<Vec<u8>, String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        on_notify(&event.value);
        if event.value[0] == cmd {
            return Ok(event.value);
        }
    }
}

async fn wait_handshake_reply_or_retry(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
    stream: &mut GattNotificationStream,
    handshake_client: &[u8],
    on_notify: &mut NotifyFn,
) -> Result<(), String> {
    let mut resent = false;
    let deadline = tokio::time::Instant::now() + HANDSHAKE_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("timeout waiting 0x36".into());
        }
        let event = match tokio::time::timeout(remaining, stream.recv()).await {
            Ok(Ok(Some(event))) => event,
            Ok(Ok(None)) => return Err("notification stream closed".into()),
            Ok(Err(e)) => return Err(format!("wait-0x36: {e}")),
            Err(_) => return Err("timeout waiting 0x36".into()),
        };
        if event.value.is_empty() {
            continue;
        }
        on_notify(&event.value);
        match event.value[0] {
            CMD_HANDSHAKE_REPLY => return Ok(()),
            crate::wire::CMD_HANDSHAKE_HOST if !resent => {
                connection
                    .write(
                        characteristic,
                        handshake_client.to_vec(),
                        WriteMode::WithResponse,
                    )
                    .await
                    .map_err(|e| format!("write-0x39-retry: {e}"))?;
                resent = true;
            }
            _ => {}
        }
    }
}
