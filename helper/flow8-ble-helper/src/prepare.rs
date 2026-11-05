//! One-shot DirectHCI controller preparation (trust certificate acknowledged).

use directhci_client::DirectHciClient;

use crate::runtime_install;

pub fn ensure_prepared(controller_id: Option<&str>) -> Result<String, String> {
    runtime_install::ensure_windows_install()?;

    let client = DirectHciClient::connect_or_start(
        "flow8-ble-helper",
        Some(env!("CARGO_PKG_VERSION").into()),
    )
    .map_err(|e| format!("connect/start DirectHCI: {e}"))?;

    let id = match controller_id {
        Some(id) => id.to_string(),
        None => {
            let list = client
                .list_controllers()
                .map_err(|e| format!("list controllers: {e}"))?;
            match list.as_slice() {
                [one] => one.id.to_string(),
                [] => return Err("no USB Bluetooth controllers found".into()),
                many => {
                    let ids: Vec<_> = many.iter().map(|c| c.id.to_string()).collect();
                    return Err(format!(
                        "multiple controllers — pass --controller <id>: {}",
                        ids.join(", ")
                    ));
                }
            }
        }
    };

    let status = client
        .controller_preparation_status(&id)
        .map_err(|e| format!("preparation status: {e}"))?;
    if status.ready {
        let _ = client.set_preferred_controller(&id);
        return Ok(id);
    }

    let prepared = client
        .prepare_controller(&id, true)
        .map_err(|e| format!("prepare_controller: {e}"))?;
    if !prepared.status.ready {
        return Err(format!(
            "prepare finished but not Ready: {:?}",
            prepared.status
        ));
    }
    client
        .set_preferred_controller(&id)
        .map_err(|e| format!("set preferred controller: {e}"))?;
    Ok(id)
}
