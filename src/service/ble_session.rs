//! BLE session public API. Backend is platform-specific:
//! - Windows: elevated DirectHCI helper (`ble_session_win`)
//! - macOS/Linux: btleplug (`ble_session_btleplug`)

use tokio::sync::mpsc;

use crate::service::ble_protocol::{BleCommand, BleEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    /// Windows DirectHCI: UAC / controller prepare before scan.
    Preparing,
    Scanning,
    Connecting,
    Subscribing,
    WaitingHandshake,
    PairingRequired,
    Authenticating,
    RequestingState,
    Active,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Disconnected => write!(f, "Disconnected"),
            SessionState::Preparing => write!(f, "Preparing"),
            SessionState::Scanning => write!(f, "Scanning"),
            SessionState::Connecting => write!(f, "Connecting"),
            SessionState::Subscribing => write!(f, "Subscribing"),
            SessionState::WaitingHandshake => write!(f, "Waiting Handshake"),
            SessionState::PairingRequired => write!(f, "Pairing Required"),
            SessionState::Authenticating => write!(f, "Authenticating"),
            SessionState::RequestingState => write!(f, "Requesting State"),
            SessionState::Active => write!(f, "Active"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionControl {
    /// User requested a BLE connection attempt.
    Connect,
    /// Stop scanning / connecting and return to idle.
    Cancel,
}

pub struct BleSessionHandle {
    pub command_tx: mpsc::Sender<BleCommand>,
    pub control_tx: mpsc::Sender<SessionControl>,
    pub event_rx: mpsc::Receiver<BleEvent>,
    pub state_rx: tokio::sync::watch::Receiver<SessionState>,
}

pub fn start_session() -> BleSessionHandle {
    #[cfg(target_os = "windows")]
    {
        crate::service::ble_session_win::start_session()
    }
    #[cfg(not(target_os = "windows"))]
    {
        crate::service::ble_session_btleplug::start_session()
    }
}
