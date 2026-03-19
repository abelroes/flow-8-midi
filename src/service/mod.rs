pub mod app_config;
pub mod ble;
pub mod midi;
pub mod midi_mapper;
pub mod single_instance;
pub mod sysex_parser;
pub mod tray;
#[cfg(any(debug_assertions, feature = "dev-tools"))]
pub mod sysex_calibration;
