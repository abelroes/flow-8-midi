use std::fmt;
use std::ops::RangeInclusive;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;

use iced::Theme;
use midir::{MidiInputConnection, MidiOutputConnection};

use super::channels::{
    AudioConnection, Bus, BusType, Channel, ChannelType, FxSlot, PhantomPower, PhantomPowerType,
};
use super::page::Page;
use crate::service::ble::{BleConnection, BleStatus};
#[cfg(any(debug_assertions, feature = "dev-tools"))]
use crate::service::sysex_calibration::CalibrationState;

pub const SNAPSHOT_COUNT: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncInterval {
    Never,
    Secs30,
    Min1,
    Min2,
    Min5,
    Min10,
    Min15,
}

impl SyncInterval {
    pub const ALL: &'static [SyncInterval] = &[
        SyncInterval::Never,
        SyncInterval::Secs30,
        SyncInterval::Min1,
        SyncInterval::Min2,
        SyncInterval::Min5,
        SyncInterval::Min10,
        SyncInterval::Min15,
    ];

    pub fn as_secs(&self) -> Option<u64> {
        match self {
            SyncInterval::Never => None,
            SyncInterval::Secs30 => Some(30),
            SyncInterval::Min1 => Some(60),
            SyncInterval::Min2 => Some(120),
            SyncInterval::Min5 => Some(300),
            SyncInterval::Min10 => Some(600),
            SyncInterval::Min15 => Some(900),
        }
    }
}

impl fmt::Display for SyncInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncInterval::Never => write!(f, "Never"),
            SyncInterval::Secs30 => write!(f, "30s"),
            SyncInterval::Min1 => write!(f, "1 min"),
            SyncInterval::Min2 => write!(f, "2 min"),
            SyncInterval::Min5 => write!(f, "5 min"),
            SyncInterval::Min10 => write!(f, "10 min"),
            SyncInterval::Min15 => write!(f, "15 min"),
        }
    }
}

pub struct FLOW8Controller {
    pub current_page: Page,
    pub theme: Theme,
    pub midi_conn: Option<MidiOutputConnection>,
    pub midi_input_conn: Option<MidiInputConnection<()>>,
    pub connected_device_name: Option<String>,
    pub connection_error: Option<String>,
    pub channels: Vec<Channel>,
    pub buses: Vec<Bus>,
    pub fx_slots: Vec<FxSlot>,
    pub sysex_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    pub last_sysex_dump: Option<Vec<u8>>,
    pub ble_status: BleStatus,
    pub ble_status_receiver: Option<mpsc::Receiver<BleStatus>>,
    pub ble_available: bool,
    pub ble_connection: Arc<Mutex<Option<BleConnection>>>,
    pub tick_counter: u32,
    pub ble_last_click: Option<Instant>,
    pub sync_last_click: Option<Instant>,
    pub sync_interval: SyncInterval,
    pub last_sync_time: Option<Instant>,
    pub snapshot_names: Vec<Option<String>>,
    pub snapshot_names_receiver: Option<mpsc::Receiver<Vec<Option<String>>>>,
    pub fx_muted: bool,
    pub snapshot_resync_at: Option<Instant>,
    #[cfg(any(debug_assertions, feature = "dev-tools"))]
    pub calibration: CalibrationState,
}

const CHANNEL_RANGE: RangeInclusive<u8> = 0..=6;
const BUS_RANGE: RangeInclusive<u8> = 7..=11;

impl FLOW8Controller {
    pub fn mark_all_synced(&mut self) {
        for ch in &mut self.channels {
            ch.mark_all_synced();
        }
        for bus in &mut self.buses {
            bus.mark_all_synced();
        }
        for fx in &mut self.fx_slots {
            fx.mark_all_synced();
        }
    }

    pub fn mark_all_unsynced(&mut self) {
        for ch in &mut self.channels {
            ch.mark_all_unsynced();
        }
        for bus in &mut self.buses {
            bus.mark_all_unsynced();
        }
        for fx in &mut self.fx_slots {
            fx.mark_all_unsynced();
        }
    }

    pub fn is_globally_synced(&self) -> bool {
        self.channels.iter().all(|c| c.is_all_synced())
            && self.buses.iter().all(|b| b.is_all_synced())
            && self.fx_slots.iter().all(|f| f.is_all_synced())
    }

    pub fn new() -> FLOW8Controller {
        FLOW8Controller {
            current_page: Page::DeviceSelect,
            theme: Theme::Dark,
            midi_conn: None,
            midi_input_conn: None,
            connected_device_name: None,
            connection_error: None,
            channels: CHANNEL_RANGE
                .map(|c_id| Channel {
                    id: c_id,
                    phantom_pwr: PhantomPower {
                        is_on: false,
                        phantom_power_type: match c_id {
                            0..=1 => PhantomPowerType::Set48v,
                            _ => PhantomPowerType::None,
                        },
                    },
                    channel_type: match c_id {
                        0..=3 => ChannelType::Mono,
                        _ => ChannelType::Stereo,
                    },
                    audio_connection: match c_id {
                        0..=1 => AudioConnection::Xlr,
                        2..=3 => AudioConnection::ComboXlr,
                        4..=5 => AudioConnection::Line,
                        _ => AudioConnection::UsbBt,
                    },
                    ..Default::default()
                })
                .collect(),
            buses: BUS_RANGE
                .map(|b_id| Bus {
                    id: b_id,
                    index: b_id - *BUS_RANGE.start(),
                    bus_type: match b_id {
                        7 => BusType::Main,
                        8..=9 => BusType::Monitor,
                        _ => BusType::Fx,
                    },
                    ..Default::default()
                })
                .collect(),
            fx_slots: vec![FxSlot::new(0), FxSlot::new(1)],
            sysex_receiver: None,
            last_sysex_dump: None,
            ble_status: BleStatus::Unavailable,
            ble_status_receiver: None,
            ble_available: false,
            ble_connection: Arc::new(Mutex::new(None)),
            tick_counter: 0,
            ble_last_click: None,
            sync_last_click: None,
            sync_interval: SyncInterval::Min2,
            last_sync_time: None,
            snapshot_names: vec![None; SNAPSHOT_COUNT],
            snapshot_names_receiver: None,
            fx_muted: false,
            snapshot_resync_at: None,
            #[cfg(any(debug_assertions, feature = "dev-tools"))]
            calibration: CalibrationState::new(),
        }
    }
}
