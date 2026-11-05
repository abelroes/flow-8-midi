//! Disposable FLOW 8 BLE POC over DirectHCI (Windows HCI / WinUSB bypass).
//!
//! Success = receive MixerState (0x38). Does not modify the main app BLE path.

#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

mod wire;

use std::env;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;

use directhci_ble::{
    BleAddress, BleCentralConfig, BleConnection, BleUuid, DirectHciBleCentral, GattCharacteristic,
    GattNotificationStream, GattService, SubscriptionMode, WriteMode,
};

use wire::{
    encode_single, load_or_create_client_id, parse_handshake_host, CMD_GET_MIXER_STATE,
    CMD_HANDSHAKE_CLIENT, CMD_HANDSHAKE_REPLY, CMD_MIXER_STATE,
};

const SERVICE_UUID: &str = "14839ad4-8d7e-415c-9a42-167340cf2339";
const CHARACTERISTIC_UUID: &str = "0034594a-a8e7-4b1a-a6b1-cd5243059a57";

const SCAN_SECS: u64 = 8;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const PAIRING_TIMEOUT: Duration = Duration::from_secs(30);
const ACK_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => {
            eprintln!("[POC] SUCCESS — MixerState (0x38) received");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[POC] FAILED: {e}");
            eprintln!(
                "[POC] If Bluetooth stayed stuck on WinUSB, use DirectHCI Control Panel recovery \
                 or `directhci.exe status` — see README.md"
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
async fn run() -> Result<(), String> {
    Err(
        "This POC only runs on Windows (DirectHCI named-pipe + WinUSB takeover). \
         Cross-compile with --target x86_64-pc-windows-gnu and run elevated on the host."
            .into(),
    )
}

#[cfg(windows)]
async fn run() -> Result<(), String> {
    let _presence = directhci_windows::process_lifecycle::enter_application()
        .map_err(|e| format!("process lifecycle: {e}"))?;

    let controller_id = parse_controller_arg();
    let client_id = load_or_create_client_id();

    eprintln!("[POC] Connecting to DirectHCI service…");
    let central = DirectHciBleCentral::connect(BleCentralConfig {
        controller_id: controller_id.clone(),
        client_name: "flow8-directhci-poc".into(),
        client_version: Some(env!("CARGO_PKG_VERSION").into()),
    })
    .await
    .map_err(|e| {
        format!(
            "stage=acquire DirectHCI: {e}. \
             Start DirectHCI Control Panel → Start Service → Prepare Controller (admin). \
             Run this POC elevated."
        )
    })?;

    let peer = match scan_for_flow8(&central).await {
        Ok(addr) => addr,
        Err(e) => {
            let _ = central.shutdown().await;
            return Err(e);
        }
    };

    eprintln!("[POC] Connecting to {peer}…");
    let connection = central
        .connect_device(peer)
        .await
        .map_err(|e| format!("stage=connect: {e}"))?;

    let result = session_until_mixer_state(&connection, &client_id).await;
    let cleanup = connection.disconnect().await;
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(e)) => {
            eprintln!("[POC] WARN disconnect/cleanup: {e}");
            Ok(())
        }
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(c)) => Err(format!("{e}; cleanup also failed: {c}")),
    }
}

fn parse_controller_arg() -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--controller" || arg == "-c" {
            return args.next();
        }
        if let Some(id) = arg.strip_prefix("--controller=") {
            return Some(id.to_string());
        }
    }
    None
}

async fn scan_for_flow8(central: &DirectHciBleCentral) -> Result<BleAddress, String> {
    eprintln!("[POC] Scanning {SCAN_SECS}s for FLOW LE…");
    let results = central
        .scan(Duration::from_secs(SCAN_SECS))
        .await
        .map_err(|e| format!("stage=scan: {e}"))?;

    let service = BleUuid::from_str(SERVICE_UUID).map_err(|e| e.to_string())?;

    for r in &results {
        let name = r.local_name.as_deref().unwrap_or("");
        eprintln!(
            "[POC]   {}  rssi={}  name={:?}  services={}",
            r.address,
            r.rssi,
            r.local_name,
            r.service_uuids.len()
        );
        let name_hit = name.to_ascii_uppercase().contains("FLOW");
        let uuid_hit = r.service_uuids.iter().any(|u| u == &service);
        if name_hit || uuid_hit {
            eprintln!("[POC] Matched FLOW peripheral {}", r.address);
            return Ok(r.address);
        }
    }

    Err(format!(
        "stage=scan: FLOW LE not found after {SCAN_SECS}s ({} ads). Power on the mixer and retry.",
        results.len()
    ))
}

async fn session_until_mixer_state(
    connection: &BleConnection,
    client_id: &[u8; 16],
) -> Result<(), String> {
    eprintln!(
        "[POC] Connected att_mtu={}",
        connection.info().att_mtu
    );

    let services = connection
        .discover()
        .await
        .map_err(|e| format!("stage=gatt-discover: {e}"))?;
    dump_gatt(&services);

    let char_uuid = BleUuid::from_str(CHARACTERISTIC_UUID).map_err(|e| e.to_string())?;
    let characteristic = services
        .iter()
        .flat_map(|s| s.characteristics.iter())
        .find(|c| c.uuid == char_uuid)
        .cloned()
        .ok_or_else(|| format!("stage=gatt-discover: characteristic {CHARACTERISTIC_UUID} missing"))?;

    eprintln!(
        "[POC] Characteristic {} handle=0x{:04X} props=0x{:02X} cccd={:?}",
        characteristic.uuid,
        characteristic.value_handle,
        characteristic.properties,
        characteristic.cccd_handle
    );

    let mut stream = enable_notifications(connection, &characteristic).await?;
    eprintln!("[POC] Notifications enabled");

    let outcome = handshake_and_state(connection, &characteristic, &mut stream, client_id).await;
    let _ = stream.close().await;
    outcome
}

fn dump_gatt(services: &[GattService]) {
    eprintln!("[POC] GATT dump ({} services):", services.len());
    for service in services {
        eprintln!(
            "[POC]   service {} handles=0x{:04X}-0x{:04X}",
            service.uuid, service.start_handle, service.end_handle
        );
        for ch in &service.characteristics {
            eprintln!(
                "[POC]     char {} value=0x{:04X} props=0x{:02X} cccd={:?}",
                ch.uuid, ch.value_handle, ch.properties, ch.cccd_handle
            );
            if ch.descriptors.is_empty() {
                eprintln!("[POC]       descriptors: (none)");
            } else {
                for d in &ch.descriptors {
                    eprintln!(
                        "[POC]       descriptor {} handle=0x{:04X}",
                        d.uuid, d.handle
                    );
                }
            }
        }
    }
}

/// Enable inbound notifications for the FLOW characteristic.
///
/// 1. Discovered CCCD -> normal subscribe
/// 2. Else try ATT write Notify enable at value_handle+1, then listen
/// 3. If that write fails (e.g. ATT 0x03 Write Not Permitted) -> passive listen only
///    (device may push HandshakeHost without a real CCCD)
async fn enable_notifications(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
) -> Result<GattNotificationStream, String> {
    const CCCD_NOTIFY_ENABLE: [u8; 2] = [0x01, 0x00];

    if let Some(cccd) = characteristic.cccd_handle {
        eprintln!("[POC] Approach A: subscribe via discovered CCCD 0x{cccd:04X}");
        return connection
            .subscribe(characteristic, SubscriptionMode::Notifications)
            .await
            .map_err(|e| format!("stage=subscribe CCCD: {e}"));
    }

    let guessed = characteristic.value_handle.saturating_add(1);
    eprintln!(
        "[POC] Approach B: cccd=None; ATT write Notify to value_handle+1 = 0x{guessed:04X}"
    );
    match connection
        .write_handle(guessed, CCCD_NOTIFY_ENABLE, WriteMode::WithResponse)
        .await
    {
        Ok(()) => {
            eprintln!("[POC] Approach B write OK; passive listen on characteristic");
            return connection
                .listen(characteristic)
                .await
                .map_err(|e| format!("stage=listen after CCCD fallback: {e}"));
        }
        Err(e) => {
            let msg = e.to_string();
            let att_write_not_permitted = msg.contains("AttErrorCode { value: 3 }")
                || msg.contains("value: 3")
                || msg.contains("0x03")
                || msg.contains("Write Not Permitted");
            eprintln!("[POC] Approach B failed: {msg}");
            if att_write_not_permitted {
                eprintln!(
                    "[POC] Approach C: ATT 0x03 on 0x{guessed:04X} (likely next char declaration); \
                     passive listen without CCCD, then wait for HandshakeHost 0x35"
                );
            } else {
                eprintln!(
                    "[POC] Approach C: falling back to passive listen without CCCD anyway"
                );
            }
        }
    }

    connection
        .listen(characteristic)
        .await
        .map_err(|e| format!("stage=passive-listen: {e}"))
}

async fn handshake_and_state(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
    stream: &mut GattNotificationStream,
    client_id: &[u8; 16],
) -> Result<(), String> {
    eprintln!("[POC] Waiting HandshakeHost (0x35)…");
    let host = tokio::time::timeout(HANDSHAKE_TIMEOUT, wait_handshake_host(stream))
        .await
        .map_err(|_| "stage=wait-0x35: timeout — Notify may not be reaching the host".to_string())?
        .map_err(|e| format!("stage=wait-0x35: {e}"))?;

    eprintln!(
        "[POC] HandshakeHost pairing_any={} proto=0x{:04X} fw=0x{:04X}",
        host.pairing_any, host.protocol_version, host.firmware_build
    );

    // HCI captures show the full framed 0x39 on ATT (19 bytes), not bare client_id.
    let handshake_client = encode_single(CMD_HANDSHAKE_CLIENT, client_id);
    eprintln!(
        "[POC] Writing HandshakeClient 0x39 ({} bytes framed)…",
        handshake_client.len()
    );
    write_handshake_client(connection, characteristic, &handshake_client).await?;

    if !host.pairing_any {
        eprintln!(
            "[POC] pairing_any=false — waiting 0x36 or new 0x35 (press mixer pairing if needed)…"
        );
        match ensure_handshake_ack(stream).await? {
            AckOutcome::Ack | AckOutcome::PairingAnyNow => {}
            AckOutcome::NeedResend => {
                eprintln!("[POC] Re-sending HandshakeClient after pairing…");
                write_handshake_client(connection, characteristic, &handshake_client).await?;
                tokio::time::timeout(ACK_TIMEOUT, wait_cmd(stream, CMD_HANDSHAKE_REPLY))
                    .await
                    .map_err(|_| "stage=wait-0x36: timeout after pairing resend".to_string())?
                    .map_err(|e| format!("stage=wait-0x36: {e}"))?;
            }
        }
    } else {
        eprintln!("[POC] Waiting HandshakeReply (0x36)…");
        wait_handshake_reply_or_retry(
            connection,
            characteristic,
            stream,
            &handshake_client,
        )
        .await?;
    }

    // Framed ack 0x36 then GetMixerState 0x37 (mirrors ble_session.rs).
    let ack = encode_single(CMD_HANDSHAKE_REPLY, &[]);
    let get_state = encode_single(CMD_GET_MIXER_STATE, &[]);
    eprintln!("[POC] Writing framed 0x36 ack + 0x37 GetMixerState…");
    connection
        .write(characteristic, ack, WriteMode::WithResponse)
        .await
        .map_err(|e| format!("stage=write-0x36-ack: {e}"))?;
    connection
        .write(characteristic, get_state, WriteMode::WithResponse)
        .await
        .map_err(|e| format!("stage=write-0x37: {e}"))?;

    eprintln!("[POC] Waiting MixerState (0x38)…");
    let first = tokio::time::timeout(HANDSHAKE_TIMEOUT, wait_cmd(stream, CMD_MIXER_STATE))
        .await
        .map_err(|_| "stage=wait-0x38: timeout".to_string())?
        .map_err(|e| format!("stage=wait-0x38: {e}"))?;

    eprintln!(
        "[POC] MixerState (0x38) received — {} bytes (first notify; multi-chunk OK)",
        first.len()
    );
    eprintln!("[POC] hex={}", hex_preview(&first, 32));
    Ok(())
}

async fn write_handshake_client(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
    packet: &[u8],
) -> Result<(), String> {
    connection
        .write(characteristic, packet.to_vec(), WriteMode::WithResponse)
        .await
        .map_err(|e| format!("stage=write-0x39: {e}"))
}

/// Wait for 0x36; if the mixer keeps advertising 0x35, re-send framed 0x39 once.
async fn wait_handshake_reply_or_retry(
    connection: &BleConnection,
    characteristic: &GattCharacteristic,
    stream: &mut GattNotificationStream,
    handshake_client: &[u8],
) -> Result<(), String> {
    let mut resent = false;
    let deadline = tokio::time::Instant::now() + HANDSHAKE_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("stage=wait-0x36: timeout".into());
        }
        let event = match tokio::time::timeout(remaining, stream.recv()).await {
            Ok(Ok(Some(event))) => event,
            Ok(Ok(None)) => return Err("notification stream closed".into()),
            Ok(Err(e)) => return Err(format!("stage=wait-0x36: {e}")),
            Err(_) => return Err("stage=wait-0x36: timeout".into()),
        };
        if event.value.is_empty() {
            continue;
        }
        match event.value[0] {
            CMD_HANDSHAKE_REPLY => return Ok(()),
            wire::CMD_HANDSHAKE_HOST if !resent => {
                eprintln!(
                    "[POC] Still seeing 0x35 after HandshakeClient — re-sending framed 0x39 once"
                );
                write_handshake_client(connection, characteristic, handshake_client).await?;
                resent = true;
            }
            other => {
                eprintln!(
                    "[POC] Ignoring notify 0x{:02X} while waiting 0x36",
                    other
                );
            }
        }
    }
}

async fn ensure_handshake_ack(
    stream: &mut GattNotificationStream,
) -> Result<AckOutcome, String> {
    let first = tokio::time::timeout(ACK_TIMEOUT, wait_ack_or_rehost(stream)).await;

    match first {
        Ok(Ok(AckOutcome::Ack)) => return Ok(AckOutcome::Ack),
        Ok(Ok(AckOutcome::PairingAnyNow)) => {
            tokio::time::timeout(ACK_TIMEOUT, wait_cmd(stream, CMD_HANDSHAKE_REPLY))
                .await
                .map_err(|_| "stage=wait-0x36: timeout after pairing_any flip".to_string())?
                .map_err(|e| format!("stage=wait-0x36: {e}"))?;
            return Ok(AckOutcome::Ack);
        }
        Ok(Ok(AckOutcome::NeedResend)) => unreachable!(),
        Ok(Err(e)) => return Err(e),
        Err(_) => {}
    }

    eprintln!(
        "[POC] No 0x36 yet — press the physical pairing button on the FLOW 8 ({}s)…",
        PAIRING_TIMEOUT.as_secs()
    );
    tokio::time::timeout(PAIRING_TIMEOUT, wait_pairing_ready(stream))
        .await
        .map_err(|_| {
            "stage=pairing: timeout (30s) — press pairing on the mixer, then retry".to_string()
        })?
        .map_err(|e| format!("stage=pairing: {e}"))?;

    Ok(AckOutcome::NeedResend)
}

enum AckOutcome {
    Ack,
    PairingAnyNow,
    NeedResend,
}

async fn wait_ack_or_rehost(
    stream: &mut GattNotificationStream,
) -> Result<AckOutcome, String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        match event.value[0] {
            CMD_HANDSHAKE_REPLY => return Ok(AckOutcome::Ack),
            wire::CMD_HANDSHAKE_HOST => {
                let host = parse_handshake_host(&event.value)?;
                if host.pairing_any {
                    return Ok(AckOutcome::PairingAnyNow);
                }
            }
            other => {
                eprintln!(
                    "[POC] Ignoring notify 0x{:02X} ({} bytes) while waiting ack",
                    other,
                    event.value.len()
                );
            }
        }
    }
}

async fn wait_pairing_ready(stream: &mut GattNotificationStream) -> Result<(), String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        if event.value[0] == wire::CMD_HANDSHAKE_HOST {
            let host = parse_handshake_host(&event.value)?;
            if host.pairing_any {
                eprintln!("[POC] pairing_any became true");
                return Ok(());
            }
        }
        if event.value[0] == CMD_HANDSHAKE_REPLY {
            eprintln!("[POC] Got 0x36 during pairing wait");
            return Ok(());
        }
    }
}

async fn wait_handshake_host(
    stream: &mut GattNotificationStream,
) -> Result<wire::HandshakeHost, String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        if event.value[0] == wire::CMD_HANDSHAKE_HOST {
            return parse_handshake_host(&event.value);
        }
        eprintln!(
            "[POC] Ignoring notify 0x{:02X} before HandshakeHost",
            event.value[0]
        );
    }
}

async fn wait_cmd(
    stream: &mut GattNotificationStream,
    cmd: u8,
) -> Result<Vec<u8>, String> {
    loop {
        let Some(event) = stream.recv().await.map_err(|e| e.to_string())? else {
            return Err("notification stream closed".into());
        };
        if event.value.is_empty() {
            continue;
        }
        if event.value[0] == cmd {
            return Ok(event.value);
        }
        eprintln!(
            "[POC] Ignoring notify 0x{:02X} while waiting 0x{:02X}",
            event.value[0], cmd
        );
    }
}

fn hex_preview(data: &[u8], max: usize) -> String {
    data.iter()
        .take(max)
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
