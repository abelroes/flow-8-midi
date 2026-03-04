use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::sync::mpsc;
use std::time::Duration;
use uuid::Uuid;

use crate::{log, log_debug, log_error, log_warn};

const SERVICE_UUID: &str = "14839ad4-8d7e-415c-9a42-167340cf2339";
const CHARACTERISTIC_UUID: &str = "0034594a-a8e7-4b1a-a6b1-cd5243059a57";
const FLOW8_NAME_KEYWORD: &str = "FLOW";
const SCAN_TIMEOUT: Duration = Duration::from_secs(12);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

const AUTH_PACKET: [u8; 19] = [
    0x39, 0x01, 0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F, 0x35,
    0x5A, 0x49, 0x5C, 0x2A,
];
const SESSION_START_PACKET: [u8; 3] = [0x37, 0x01, 0x38];
const CONFIG_REQUEST_PACKET: [u8; 3] = [0x07, 0x01, 0x08];
const DUMP_TRIGGER_PACKET: [u8; 3] = [0x4B, 0x01, 0x4C];

const SNAPSHOT_RESPONSE_TYPE: u8 = 0x27;
const STATE_DUMP_TYPE: u8 = 0x38;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleStatus {
    Unavailable,
    Scanning,
    Connecting,
    Authenticating,
    Connected,
    Disconnected,
    Error,
}

impl std::fmt::Display for BleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BleStatus::Unavailable => write!(f, "BT Unavailable"),
            BleStatus::Scanning => write!(f, "BT Scanning..."),
            BleStatus::Connecting => write!(f, "BT Connecting..."),
            BleStatus::Authenticating => write!(f, "BT Authenticating..."),
            BleStatus::Connected => write!(f, "BT Connected"),
            BleStatus::Disconnected => write!(f, "BT Disconnected"),
            BleStatus::Error => write!(f, "BT Error"),
        }
    }
}

pub struct BleConnection {
    pub peripheral: Peripheral,
    pub characteristic: Characteristic,
}

fn service_uuid() -> Uuid {
    Uuid::parse_str(SERVICE_UUID).unwrap()
}

fn characteristic_uuid() -> Uuid {
    Uuid::parse_str(CHARACTERISTIC_UUID).unwrap()
}

pub fn is_ble_available() -> bool {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    rt.block_on(async {
        let manager = match Manager::new().await {
            Ok(m) => m,
            Err(_) => return false,
        };
        let adapters = match manager.adapters().await {
            Ok(a) => a,
            Err(_) => return false,
        };
        !adapters.is_empty()
    })
}

async fn get_adapter() -> Result<Adapter, String> {
    let manager = Manager::new()
        .await
        .map_err(|e| format!("BLE manager error: {}", e))?;

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("BLE adapter list error: {}", e))?;

    adapters
        .into_iter()
        .next()
        .ok_or_else(|| "No BLE adapter found".to_string())
}

async fn scan_for_flow8(adapter: &Adapter) -> Result<Peripheral, String> {
    log!("[BLE] Scanning for FLOW 8 ({}s timeout)...", SCAN_TIMEOUT.as_secs());

    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| format!("Scan start error: {}", e))?;

    tokio::time::sleep(SCAN_TIMEOUT).await;

    adapter.stop_scan().await.ok();

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripheral list error: {}", e))?;

    log_debug!("[BLE] Found {} device(s) during scan", peripherals.len());

    let mut name_match: Option<Peripheral> = None;
    let target_svc = service_uuid();

    for p in peripherals {
        if let Ok(Some(props)) = p.properties().await {
            let label = props
                .local_name
                .as_deref()
                .unwrap_or("(unnamed)");
            let addr = format!("{:?}", props.address);

            log_debug!("[BLE]   device: \"{}\" addr={}", label, addr);

            if name_match.is_none() {
                if let Some(ref name) = props.local_name {
                    if name.to_uppercase().contains(FLOW8_NAME_KEYWORD) {
                        log!("[BLE] Matched by name: \"{}\"", name);
                        name_match = Some(p.clone());
                        continue;
                    }
                }

                if props.services.contains(&target_svc) {
                    log!("[BLE] Matched by Service UUID: \"{}\" addr={}", label, addr);
                    name_match = Some(p.clone());
                }
            }
        }
    }

    name_match.ok_or_else(|| "FLOW 8 LE not found during scan".to_string())
}

async fn connect_peripheral(peripheral: &Peripheral) -> Result<(), String> {
    log!("[BLE] Connecting ({}s timeout)...", CONNECT_TIMEOUT.as_secs());

    tokio::time::timeout(CONNECT_TIMEOUT, peripheral.connect())
        .await
        .map_err(|_| "Connection timed out".to_string())?
        .map_err(|e| format!("Connection failed: {}", e))?;

    log!("[BLE] Connected. Discovering services...");
    peripheral
        .discover_services()
        .await
        .map_err(|e| format!("Service discovery failed: {}", e))?;

    Ok(())
}

const MAX_CONNECT_RETRIES: u32 = 2;

async fn connect_and_auth(
    peripheral: &Peripheral,
    status_tx: &mpsc::Sender<BleStatus>,
) -> Result<Characteristic, String> {
    let mut last_err = String::new();

    for attempt in 0..=MAX_CONNECT_RETRIES {
        if attempt > 0 {
            log_warn!("[BLE] Retry {}/{}...", attempt, MAX_CONNECT_RETRIES);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let _ = status_tx.send(BleStatus::Connecting);

        match connect_peripheral(peripheral).await {
            Ok(_) => {}
            Err(e) => {
                log_warn!("[BLE] Connect attempt failed: {}", e);
                peripheral.disconnect().await.ok();
                last_err = e;
                continue;
            }
        }

        let target_uuid = characteristic_uuid();
        let characteristic = match peripheral
            .characteristics()
            .into_iter()
            .find(|c| c.uuid == target_uuid)
        {
            Some(c) => c,
            None => {
                last_err = "FLOW 8 BLE characteristic not found".to_string();
                log_warn!("[BLE] {}", last_err);
                peripheral.disconnect().await.ok();
                continue;
            }
        };

        log!("[BLE] Characteristic found.");
        let _ = status_tx.send(BleStatus::Authenticating);

        log!("[BLE] Sending auth key...");
        if let Err(e) = peripheral
            .write(&characteristic, &AUTH_PACKET, WriteType::WithResponse)
            .await
        {
            log_warn!("[BLE] Auth write failed: {}", e);
            peripheral.disconnect().await.ok();
            last_err = format!("Auth write failed: {}", e);
            continue;
        }

        tokio::time::sleep(Duration::from_millis(300)).await;

        log!("[BLE] Sending session start...");
        if let Err(e) = peripheral
            .write(&characteristic, &SESSION_START_PACKET, WriteType::WithResponse)
            .await
        {
            log_warn!("[BLE] Session start write failed: {}", e);
            peripheral.disconnect().await.ok();
            last_err = format!("Session start write failed: {}", e);
            continue;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        log!("[BLE] Authenticated and session started");
        return Ok(characteristic);
    }

    Err(last_err)
}

fn parse_snapshot_names(data: &[u8]) -> Vec<Option<String>> {
    let mut names: Vec<Option<String>> = Vec::new();

    if data.len() < 3 || data[0] != SNAPSHOT_RESPONSE_TYPE {
        log_warn!(
            "[BLE] Unexpected snapshot response: len={}, first_byte=0x{:02X}",
            data.len(),
            data.first().copied().unwrap_or(0)
        );
        return names;
    }

    let payload = &data[2..data.len().saturating_sub(1)];
    let mut pos = 0;

    log_debug!(
        "[BLE] Parsing snapshot payload: {} bytes (full packet {} bytes)",
        payload.len(),
        data.len()
    );

    while pos < payload.len() && names.len() < 15 {
        let len = payload[pos] as usize;
        pos += 1;

        if len == 0 {
            names.push(None);
        } else if pos + len <= payload.len() {
            match std::str::from_utf8(&payload[pos..pos + len]) {
                Ok(s) => {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        names.push(None);
                    } else {
                        log_debug!("[BLE] Snapshot {}: \"{}\"", names.len() + 1, trimmed);
                        names.push(Some(trimmed.to_string()));
                    }
                }
                Err(_) => {
                    log_debug!(
                        "[BLE] Snapshot {}: invalid UTF-8 ({} bytes)",
                        names.len() + 1,
                        len
                    );
                    names.push(None);
                }
            }
            pos += len;
        } else {
            log_debug!("[BLE] Snapshot parse ended: pos={} + len={} > payload.len()={}", pos - 1, len, payload.len());
            break;
        }
    }

    names
}

async fn fetch_snapshot_names(
    peripheral: &Peripheral,
    characteristic: &Characteristic,
) -> Result<Vec<Option<String>>, String> {
    use tokio_stream::StreamExt;

    log!("[BLE] Subscribing to notifications for snapshot fetch...");
    let mut subscribed = false;
    for attempt in 0..3u64 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
        }
        match peripheral.subscribe(characteristic).await {
            Ok(_) => {
                subscribed = true;
                break;
            }
            Err(e) => log_warn!("[BLE] Subscribe attempt {}/3: {}", attempt + 1, e),
        }
    }
    if !subscribed {
        log_warn!("[BLE] Subscribe failed after 3 attempts, snapshot names unavailable");
        return Ok(Vec::new());
    }

    let mut notification_stream = match peripheral.notifications().await {
        Ok(s) => s,
        Err(e) => {
            log_warn!("[BLE] Notification stream error: {}", e);
            peripheral.unsubscribe(characteristic).await.ok();
            return Ok(Vec::new());
        }
    };

    log!("[BLE] Draining state dump (0x38)...");
    let drain_result = tokio::time::timeout(Duration::from_secs(3), async {
        let mut dump_chunks = 0u32;
        while let Some(notification) = notification_stream.next().await {
            if !notification.value.is_empty() {
                let ptype = notification.value[0];
                log_debug!(
                    "[BLE] Notification: type=0x{:02X}, len={}",
                    ptype,
                    notification.value.len()
                );
                if ptype == STATE_DUMP_TYPE {
                    dump_chunks += 1;
                    if dump_chunks >= 4 {
                        return dump_chunks;
                    }
                }
            }
        }
        dump_chunks
    })
    .await;

    match drain_result {
        Ok(n) => log!("[BLE] State dump received ({} chunks)", n),
        Err(_) => log_warn!("[BLE] State dump wait timed out, proceeding anyway"),
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    log!("[BLE] Requesting snapshot names (0x07)...");
    if let Err(e) = peripheral
        .write(characteristic, &CONFIG_REQUEST_PACKET, WriteType::WithResponse)
        .await
    {
        log_warn!("[BLE] Config request write failed: {}", e);
        peripheral.unsubscribe(characteristic).await.ok();
        return Ok(Vec::new());
    }

    let result = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(notification) = notification_stream.next().await {
            if !notification.value.is_empty() {
                let ptype = notification.value[0];
                log_debug!(
                    "[BLE] Notification: type=0x{:02X}, len={}",
                    ptype,
                    notification.value.len()
                );
                if ptype == SNAPSHOT_RESPONSE_TYPE {
                    return Some(parse_snapshot_names(&notification.value));
                }
            }
        }
        None
    })
    .await;

    peripheral.unsubscribe(characteristic).await.ok();

    match result {
        Ok(Some(names)) => {
            let named_count = names.iter().filter(|n| n.is_some()).count();
            log!(
                "[BLE] Parsed {} snapshot slots ({} with names)",
                names.len(),
                named_count
            );
            Ok(names)
        }
        Ok(None) => {
            log_warn!("[BLE] Notification stream ended without snapshot response");
            Ok(Vec::new())
        }
        Err(_) => {
            log_warn!("[BLE] Timeout waiting for snapshot names (5s)");
            Ok(Vec::new())
        }
    }
}

pub fn connect_flow8_ble(
    status_tx: mpsc::Sender<BleStatus>,
    snapshot_tx: mpsc::Sender<Vec<Option<String>>>,
) -> Result<BleConnection, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Tokio runtime error: {}", e))?;

    rt.block_on(async {
        let _ = status_tx.send(BleStatus::Scanning);
        let adapter = get_adapter().await?;

        let peripheral = match scan_for_flow8(&adapter).await {
            Ok(p) => p,
            Err(e) => {
                let _ = status_tx.send(BleStatus::Error);
                return Err(e);
            }
        };

        let characteristic = match connect_and_auth(&peripheral, &status_tx).await {
            Ok(c) => c,
            Err(e) => {
                let _ = status_tx.send(BleStatus::Error);
                return Err(e);
            }
        };

        let _ = status_tx.send(BleStatus::Connected);

        match fetch_snapshot_names(&peripheral, &characteristic).await {
            Ok(names) if !names.is_empty() => {
                let _ = snapshot_tx.send(names);
            }
            Ok(_) => log_warn!("[BLE] No snapshot names received"),
            Err(e) => log_warn!("[BLE] Failed to get snapshot names: {}", e),
        }

        Ok(BleConnection {
            peripheral,
            characteristic,
        })
    })
}

pub fn send_dump_trigger(conn: &BleConnection) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Tokio runtime error: {}", e))?;

    rt.block_on(async {
        log!("[BLE] Sending dump trigger (0x4B)...");
        conn.peripheral
            .write(
                &conn.characteristic,
                &DUMP_TRIGGER_PACKET,
                WriteType::WithResponse,
            )
            .await
            .map_err(|e| format!("Dump trigger write failed: {}", e))?;

        log!("[BLE] Dump trigger sent. SysEx should arrive via USB MIDI.");
        Ok(())
    })
}

pub fn disconnect(conn: &BleConnection) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log_error!("[BLE] Could not create runtime for disconnect: {}", e);
            return;
        }
    };

    rt.block_on(async {
        if let Err(e) = conn.peripheral.disconnect().await {
            log_warn!("[BLE] Disconnect error: {}", e);
        } else {
            log!("[BLE] Disconnected");
        }
    });
}
