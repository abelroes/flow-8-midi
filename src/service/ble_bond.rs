/// Windows OS BLE pairing helpers.
///
/// Important findings from GPD Win Max 2 logs:
/// - Auto-unpair/re-pair cycles leave the mixer flapping Connected/Not Connected
///   and often make CCCD worse (ATT 0x03 / HRESULT 0x80650003).
/// - Default PairAsync frequently returns status 19; Custom ConfirmOnly works.
/// - ProtectionLevel often stays None even after a "successful" pair — FLOW 8
///   does not appear to negotiate encrypted bonding the way Windows prefers.
/// - CCCD enable must happen on a *fresh* GATT connection after the OS bond
///   exists (known WinRT quirk: CCCD write right after pairing in the same
///   connection often fails; the next connection succeeds).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondOutcome {
    AlreadyPaired,
    NewlyPaired,
    Skipped,
}

#[cfg(target_os = "windows")]
fn bond_log(msg: &str) {
    crate::logger::log(msg);
}

#[cfg(target_os = "windows")]
fn bond_warn(msg: &str) {
    crate::logger::log_with_level(crate::logger::LogLevel::Warn, msg);
}

/// Ensure the device is paired in Windows. Never unpairs existing bonds.
/// Also probes GATT descriptors (looking for standard CCCD 0x2902).
#[cfg(target_os = "windows")]
pub async fn ensure_paired(address: u64) -> Result<BondOutcome, String> {
    use tokio::task;
    task::spawn_blocking(move || windows_ensure_paired(address))
        .await
        .map_err(|e| format!("Spawn blocking: {}", e))?
}

#[cfg(target_os = "windows")]
pub async fn prepare_encrypted_notify(address: u64) -> Result<BondOutcome, String> {
    ensure_paired(address).await
}

#[cfg(target_os = "windows")]
pub async fn ensure_bonded(address: u64) -> Result<BondOutcome, String> {
    ensure_paired(address).await
}

#[cfg(target_os = "windows")]
pub async fn force_rebond(address: u64) -> Result<BondOutcome, String> {
    use tokio::task;
    task::spawn_blocking(move || {
        bond_warn("[BLE_BOND] force_rebond requested — unpair then pair once");
        let device = open_device_by_address(address)?;
        let pairing = device_pairing(&device)?;
        if pairing.IsPaired().unwrap_or(false) {
            windows_unpair(&pairing)?;
        }
        let _ = device.Close();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        windows_ensure_paired(address)
    })
    .await
    .map_err(|e| format!("Spawn blocking: {}", e))?
}

#[cfg(target_os = "windows")]
fn windows_ensure_paired(address: u64) -> Result<BondOutcome, String> {
    use std::thread;
    use std::time::Duration;
    use windows::Devices::Enumeration::DevicePairingProtectionLevel;

    let device = open_device_by_address(address)?;
    let pairing = device_pairing(&device)?;
    let is_paired = pairing.IsPaired().unwrap_or(false);
    let level = pairing
        .ProtectionLevel()
        .unwrap_or(DevicePairingProtectionLevel::None);
    bond_log(&format!(
        "[BLE_BOND] ensure_paired start: IsPaired={} ProtectionLevel={:?}",
        is_paired, level
    ));

    // Probe descriptors while we have a device handle (best-effort diagnostics).
    if let Err(e) = probe_gatt_descriptors(&device) {
        bond_warn(&format!("[BLE_BOND] GATT probe: {}", e));
    }

    if is_paired {
        let _ = device.Close();
        bond_log("[BLE_BOND] Already paired — leave bond intact");
        return Ok(BondOutcome::AlreadyPaired);
    }

    if !pairing.CanPair().unwrap_or(false) {
        let _ = device.Close();
        return Err(
            "Windows reports CanPair=false. Remove \"FLOW LE\" from Windows Bluetooth settings, then retry."
                .into(),
        );
    }

    bond_log("[BLE_BOND] Custom ConfirmOnly pair (no unpair)");
    windows_custom_pair(&pairing)?;
    let level_after = pairing
        .ProtectionLevel()
        .unwrap_or(DevicePairingProtectionLevel::None);
    bond_log(&format!(
        "[BLE_BOND] Paired OK ProtectionLevelUsed/now={:?}",
        level_after
    ));
    let _ = device.Close();
    thread::sleep(Duration::from_millis(1000));

    // Re-probe on a fresh post-pair connection — descriptors may be hidden
    // until the OS finishes bonding.
    bond_log("[BLE_BOND] Re-probe descriptors after pair…");
    let device2 = open_device_by_address(address)?;
    if let Err(e) = probe_gatt_descriptors(&device2) {
        bond_warn(&format!("[BLE_BOND] Post-pair GATT probe: {}", e));
    }
    let _ = device2.Close();
    thread::sleep(Duration::from_millis(500));
    Ok(BondOutcome::NewlyPaired)
}

/// Probe GATT descriptors while another stack (btleplug) may already hold the link.
#[cfg(target_os = "windows")]
pub async fn probe_descriptors(address: u64) -> Result<(), String> {
    use tokio::task;
    task::spawn_blocking(move || {
        let device = open_device_by_address(address)?;
        let result = probe_gatt_descriptors(&device);
        let _ = device.Close();
        result
    })
    .await
    .map_err(|e| format!("Spawn blocking: {}", e))?
}

#[cfg(target_os = "windows")]
fn probe_gatt_descriptors(
    device: &windows::Devices::Bluetooth::BluetoothLEDevice,
) -> Result<(), String> {
    use windows::Devices::Bluetooth::GenericAttributeProfile::GattCommunicationStatus;
    use windows::Devices::Bluetooth::BluetoothCacheMode;

    const SERVICE_UUID: &str = "14839ad4-8d7e-415c-9a42-167340cf2339";
    const CHARACTERISTIC_UUID: &str = "0034594a-a8e7-4b1a-a6b1-cd5243059a57";
    const CCCD_UUID: &str = "00002902-0000-1000-8000-00805f9b34fb";

    let access = device
        .RequestAccessAsync()
        .map_err(|e| format!("RequestAccessAsync: {}", e))?
        .get()
        .map_err(|e| format!("RequestAccess get: {}", e))?;
    bond_log(&format!("[BLE_BOND] RequestAccess={:?}", access));

    let svc_guid = uuid_to_guid(SERVICE_UUID)?;
    let char_guid = uuid_to_guid(CHARACTERISTIC_UUID)?;
    let cccd_guid = uuid_to_guid(CCCD_UUID)?;

    let services = device
        .GetGattServicesForUuidWithCacheModeAsync(svc_guid, BluetoothCacheMode::Uncached)
        .map_err(|e| format!("GetGattServicesForUuid: {}", e))?
        .get()
        .map_err(|e| format!("GetGattServices get: {}", e))?;

    let svc_status = services
        .Status()
        .map_err(|e| format!("Services status: {}", e))?;
    if svc_status != GattCommunicationStatus::Success {
        return Err(format!("GetGattServices status={:?}", svc_status));
    }

    let service_list = services.Services().map_err(|e| format!("Services: {}", e))?;
    if service_list.Size().unwrap_or(0) == 0 {
        return Err("FLOW 8 GATT service not found".into());
    }
    let service = service_list
        .GetAt(0)
        .map_err(|e| format!("Service GetAt: {}", e))?;

    let chars = service
        .GetCharacteristicsForUuidWithCacheModeAsync(char_guid, BluetoothCacheMode::Uncached)
        .map_err(|e| format!("GetCharacteristicsForUuid: {}", e))?
        .get()
        .map_err(|e| format!("GetCharacteristics get: {}", e))?;

    let char_status = chars.Status().map_err(|e| format!("Chars status: {}", e))?;
    if char_status != GattCommunicationStatus::Success {
        let _ = service.Close();
        return Err(format!("GetCharacteristics status={:?}", char_status));
    }

    let char_list = chars
        .Characteristics()
        .map_err(|e| format!("Characteristics: {}", e))?;
    if char_list.Size().unwrap_or(0) == 0 {
        let _ = service.Close();
        return Err("FLOW 8 characteristic not found".into());
    }
    let characteristic = char_list
        .GetAt(0)
        .map_err(|e| format!("Characteristic GetAt: {}", e))?;

    if let Ok(props) = characteristic.CharacteristicProperties() {
        bond_log(&format!(
            "[BLE_BOND] CharacteristicProperties={:?} (Notify=0x10 Write=0x08)",
            props
        ));
    }

    // List ALL descriptors — Windows requires standard CCCD UUID 0x2902.
    let descs = characteristic
        .GetDescriptorsWithCacheModeAsync(BluetoothCacheMode::Uncached)
        .map_err(|e| format!("GetDescriptors: {}", e))?
        .get()
        .map_err(|e| format!("GetDescriptors get: {}", e))?;

    let desc_status = descs.Status().map_err(|e| format!("Desc status: {}", e))?;
    bond_log(&format!(
        "[BLE_BOND] GetDescriptors status={:?}",
        desc_status
    ));

    if desc_status == GattCommunicationStatus::Success {
        let list = descs.Descriptors().map_err(|e| format!("Descriptors: {}", e))?;
        let n = list.Size().unwrap_or(0);
        bond_log(&format!("[BLE_BOND] Descriptor count={}", n));
        let mut found_cccd = false;
        for i in 0..n {
            if let Ok(d) = list.GetAt(i) {
                let uuid = d
                    .Uuid()
                    .map(|u| format!("{:?}", u))
                    .unwrap_or_else(|_| "?".into());
                let handle = d
                    .AttributeHandle()
                    .map(|h| format!("0x{:04X}", h))
                    .unwrap_or_else(|_| "?".into());
                bond_log(&format!(
                    "[BLE_BOND]   descriptor[{}] uuid={} handle={}",
                    i, uuid, handle
                ));
                if let Ok(u) = d.Uuid() {
                    if u == cccd_guid {
                        found_cccd = true;
                    }
                }
            }
        }
        if found_cccd {
            bond_log("[BLE_BOND] Standard CCCD 0x2902 PRESENT");
        } else {
            bond_warn(
                "[BLE_BOND] Standard CCCD 0x2902 MISSING — Windows UWP rejects notify on non-compliant peripherals (Android still works)",
            );
        }
    }

    // Also query specifically for 0x2902.
    let cccd_result = characteristic
        .GetDescriptorsForUuidWithCacheModeAsync(cccd_guid, BluetoothCacheMode::Uncached)
        .map_err(|e| format!("GetDescriptorsForUuid CCCD: {}", e))?
        .get()
        .map_err(|e| format!("GetDescriptorsForUuid CCCD get: {}", e))?;
    let cccd_status = cccd_result
        .Status()
        .map_err(|e| format!("CCCD query status: {}", e))?;
    let cccd_count = cccd_result
        .Descriptors()
        .ok()
        .and_then(|d| d.Size().ok())
        .unwrap_or(0);
    bond_log(&format!(
        "[BLE_BOND] CCCD 0x2902 query status={:?} count={}",
        cccd_status, cccd_count
    ));

    let _ = service.Close();
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_device_by_address(
    address: u64,
) -> Result<windows::Devices::Bluetooth::BluetoothLEDevice, String> {
    use windows::Devices::Bluetooth::BluetoothLEDevice;

    bond_log(&format!(
        "[BLE_BOND] Open FromBluetoothAddress 0x{:012X}",
        address & 0xFFFF_FFFF_FFFF
    ));
    let device = BluetoothLEDevice::FromBluetoothAddressAsync(address)
        .map_err(|e| format!("FromBluetoothAddressAsync: {}", e))?
        .get()
        .map_err(|e| format!("Get device: {}", e))?;
    let name = device
        .Name()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "?".into());
    let conn = device
        .ConnectionStatus()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|_| "?".into());
    bond_log(&format!(
        "[BLE_BOND] Device name=\"{}\" ConnectionStatus={}",
        name, conn
    ));
    Ok(device)
}

#[cfg(target_os = "windows")]
fn device_pairing(
    device: &windows::Devices::Bluetooth::BluetoothLEDevice,
) -> Result<windows::Devices::Enumeration::DeviceInformationPairing, String> {
    device
        .DeviceInformation()
        .map_err(|e| format!("DeviceInformation: {}", e))?
        .Pairing()
        .map_err(|e| format!("Pairing: {}", e))
}

#[cfg(target_os = "windows")]
fn windows_unpair(
    pairing: &windows::Devices::Enumeration::DeviceInformationPairing,
) -> Result<(), String> {
    use windows::Devices::Enumeration::DeviceUnpairingResultStatus;

    bond_log("[BLE_BOND] UnpairAsync…");
    let result = pairing
        .UnpairAsync()
        .map_err(|e| format!("UnpairAsync: {}", e))?
        .get()
        .map_err(|e| format!("UnpairAsync get: {}", e))?;
    let status = result.Status().map_err(|e| format!("Unpair status: {}", e))?;
    match status {
        DeviceUnpairingResultStatus::Unpaired | DeviceUnpairingResultStatus::AlreadyUnpaired => {
            bond_log(&format!("[BLE_BOND] Unpair OK ({:?})", status));
            Ok(())
        }
        other => Err(format!("Unpair failed: {:?}", other)),
    }
}

#[cfg(target_os = "windows")]
fn windows_custom_pair(
    pairing: &windows::Devices::Enumeration::DeviceInformationPairing,
) -> Result<(), String> {
    use windows::core::Result as WinResult;
    use windows::Devices::Enumeration::{
        DeviceInformationCustomPairing, DevicePairingKinds, DevicePairingProtectionLevel,
        DevicePairingRequestedEventArgs, DevicePairingResultStatus,
    };
    use windows::Foundation::TypedEventHandler;

    let custom = pairing
        .Custom()
        .map_err(|e| format!("Custom pairing: {}", e))?;

    let handler = TypedEventHandler::<
        DeviceInformationCustomPairing,
        DevicePairingRequestedEventArgs,
    >::new(|_sender, args| -> WinResult<()> {
        if let Some(args) = args {
            // ConfirmOnly / no PIN — Accept() is required for the ceremony.
            args.Accept()?;
        }
        Ok(())
    });

    // Prefer Encryption first (MS guidance), fall back to None for devices that
    // only support Just Works without LE Secure Connections.
    let attempts = [
        (
            DevicePairingKinds::ConfirmOnly,
            DevicePairingProtectionLevel::Encryption,
        ),
        (
            DevicePairingKinds::ConfirmOnly,
            DevicePairingProtectionLevel::None,
        ),
    ];

    for (kinds, level) in attempts {
        bond_log(&format!(
            "[BLE_BOND] Custom PairAsync({:?}, {:?})",
            kinds, level
        ));
        let token = custom
            .PairingRequested(&handler)
            .map_err(|e| format!("PairingRequested: {}", e))?;
        let result = custom
            .PairWithProtectionLevelAsync(kinds, level)
            .map_err(|e| format!("Custom PairAsync: {}", e))?
            .get()
            .map_err(|e| format!("Custom PairAsync get: {}", e))?;
        let _ = custom.RemovePairingRequested(token);

        let status = result.Status().map_err(|e| format!("Status: {}", e))?;
        let used = result
            .ProtectionLevelUsed()
            .map(|l| format!("{:?}", l))
            .unwrap_or_else(|_| "?".into());
        bond_log(&format!(
            "[BLE_BOND] Custom status={:?} ProtectionLevelUsed={}",
            status, used
        ));
        if matches!(
            status,
            DevicePairingResultStatus::Paired | DevicePairingResultStatus::AlreadyPaired
        ) {
            return Ok(());
        }
    }

    Err("Custom pairing failed for Encryption and None".into())
}

#[cfg(target_os = "windows")]
fn uuid_to_guid(uuid: &str) -> Result<windows::core::GUID, String> {
    let u = uuid::Uuid::parse_str(uuid).map_err(|e| format!("UUID parse: {}", e))?;
    let b = u.as_bytes();
    Ok(windows::core::GUID {
        data1: u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        data2: u16::from_be_bytes([b[4], b[5]]),
        data3: u16::from_be_bytes([b[6], b[7]]),
        data4: [b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]],
    })
}

/// True when BlueZ (or similar) reports a missing BR/EDR or LE link key.
pub fn is_link_key_missing(err: &str) -> bool {
    let s = err.to_lowercase();
    s.contains("key-missing")
        || s.contains("key_missing")
        || s.contains("brconnectionkeymissing")
        || s.contains("leconnectionkeymissing")
        || s.contains("link key missing")
}

/// True when BlueZ Device1 vanished (e.g. after RemoveDevice) and Connect is invalid.
pub fn is_stale_device_error(err: &str) -> bool {
    let s = err.to_lowercase();
    s.contains("doesn't exist")
        || s.contains("does not exist")
        || s.contains("unknownobject")
        || s.contains("unknown object")
        || (s.contains("org.bluez.device1") && s.contains("connect"))
}

/// Transient GATT failures seen right after a fresh BlueZ link (often ATT 0x0E).
/// Same class of "fresh connection not ready" as Windows CCCD 0x80650003.
pub fn is_transient_gatt_error(err: &str) -> bool {
    let s = err.to_lowercase();
    s.contains("0x0e")
        || s.contains("unlikely error")
        || s.contains("att error: 0x0e")
        || s.contains("0x0f")
        || s.contains("insufficient encryption")
        || s.contains("insufficient authentication")
}

/// Errors where an automatic full session retry is worth it (Linux BlueZ flakiness).
pub fn is_recoverable_session_error(err: &str) -> bool {
    is_link_key_missing(err) || is_stale_device_error(err) || is_transient_gatt_error(err)
}

#[cfg(not(target_os = "windows"))]
pub async fn ensure_paired(_address: u64) -> Result<BondOutcome, String> {
    Ok(BondOutcome::Skipped)
}

#[cfg(not(target_os = "windows"))]
pub async fn prepare_encrypted_notify(_address: u64) -> Result<BondOutcome, String> {
    Ok(BondOutcome::Skipped)
}

#[cfg(not(target_os = "windows"))]
pub async fn ensure_bonded(_address: u64) -> Result<BondOutcome, String> {
    Ok(BondOutcome::Skipped)
}

/// Linux: remove the BlueZ device entry (clears stale bonding / link keys).
/// Other non-Windows platforms: no-op.
#[cfg(target_os = "linux")]
pub async fn force_rebond(address: u64) -> Result<BondOutcome, String> {
    use tokio::task;
    task::spawn_blocking(move || linux_remove_bond(address))
        .await
        .map_err(|e| format!("Spawn blocking: {}", e))?
}

#[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
pub async fn force_rebond(_address: u64) -> Result<BondOutcome, String> {
    Ok(BondOutcome::Skipped)
}

#[cfg(not(target_os = "windows"))]
pub async fn probe_descriptors(_address: u64) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn bond_log(msg: &str) {
    crate::logger::log(msg);
}

#[cfg(target_os = "linux")]
fn bond_warn(msg: &str) {
    crate::logger::log_with_level(crate::logger::LogLevel::Warn, msg);
}

#[cfg(target_os = "linux")]
fn address_to_mac(address: u64) -> String {
    let a = address & 0xFFFF_FFFF_FFFF;
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        (a >> 40) & 0xff,
        (a >> 32) & 0xff,
        (a >> 24) & 0xff,
        (a >> 16) & 0xff,
        (a >> 8) & 0xff,
        a & 0xff,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bluez_key_missing_errors() {
        assert!(is_link_key_missing("Connect: br-connection-key-missing"));
        assert!(is_link_key_missing("org.bluez.Error.BrConnectionKeyMissing"));
        assert!(is_link_key_missing("le-connection-key-missing"));
        assert!(!is_link_key_missing("Connection timed out"));
        assert!(!is_link_key_missing("Subscribe failed"));
    }

    #[test]
    fn detects_stale_and_transient_gatt_errors() {
        assert!(is_stale_device_error(
            "Connect: Method 'Connect' with signature '' on interface 'org.bluez.Device1' doesn't exist"
        ));
        assert!(is_transient_gatt_error(
            "BLE write: Operation failed with ATT error: 0x0e"
        ));
        assert!(is_recoverable_session_error(
            "BLE write: Operation failed with ATT error: 0x0e"
        ));
        assert!(!is_stale_device_error("Subscribe failed: timeout"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn formats_bd_addr_for_bluez() {
        assert_eq!(address_to_mac(0x1122_3344_5566), "11:22:33:44:55:66");
        assert_eq!(address_to_mac(0xAABB_CCDDEEFF), "AA:BB:CC:DD:EE:FF");
    }
}

/// Drop BlueZ cached device + bonding for the given BD_ADDR via Adapter1.RemoveDevice.
#[cfg(target_os = "linux")]
fn linux_remove_bond(address: u64) -> Result<BondOutcome, String> {
    use dbus::arg::RefArg;
    use dbus::blocking::stdintf::org_freedesktop_dbus::ObjectManager;
    use dbus::blocking::Connection;
    use std::time::Duration;

    let mac = address_to_mac(address);
    bond_log(&format!(
        "[BLE_BOND] Linux force_rebond: RemoveDevice for {}",
        mac
    ));

    let conn = Connection::new_system()
        .map_err(|e| format!("D-Bus system bus: {}. Is bluetoothd running?", e))?;
    let root = conn.with_proxy("org.bluez", "/", Duration::from_secs(5));
    let objects = root
        .get_managed_objects()
        .map_err(|e| format!("BlueZ GetManagedObjects: {}", e))?;

    let mut removed = 0u32;
    for (path, interfaces) in &objects {
        let Some(props) = interfaces.get("org.bluez.Device1") else {
            continue;
        };
        let Some(addr) = props.get("Address").and_then(|v| v.as_str()) else {
            continue;
        };
        if !addr.eq_ignore_ascii_case(&mac) {
            continue;
        }

        let path_str = path.to_string();
        let adapter_path = path_str
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_string())
            .unwrap_or_else(|| "/org/bluez/hci0".to_string());

        bond_log(&format!(
            "[BLE_BOND] RemoveDevice {} on {}",
            path_str, adapter_path
        ));

        let adapter = conn.with_proxy("org.bluez", adapter_path, Duration::from_secs(8));
        adapter
            .method_call::<(), _, _, _>("org.bluez.Adapter1", "RemoveDevice", (path.clone(),))
            .map_err(|e| {
                format!(
                    "BlueZ RemoveDevice failed ({}). Clear the FLOW device in system Bluetooth settings, then retry.",
                    e
                )
            })?;
        removed += 1;
    }

    if removed == 0 {
        bond_warn(&format!(
            "[BLE_BOND] No BlueZ Device1 entry for {} — nothing to clear",
            mac
        ));
        return Ok(BondOutcome::Skipped);
    }

    bond_log(&format!(
        "[BLE_BOND] Cleared {} BlueZ device entr(y/ies); reconnect required",
        removed
    ));
    Ok(BondOutcome::NewlyPaired)
}
