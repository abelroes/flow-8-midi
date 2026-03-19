#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod logger;
mod model;
pub mod service;
mod view;

use view::{
    device_select_page::view_device_select, eq_page::view_eq,
    fx_page::view_fx, mixer_page::view_mixer,
    nav_bar::view_nav_bar, sends_page::view_sends, settings_page::view_settings,
    snapshots_page::view_snapshots,
};
use image::load_from_memory;
use service::ble;
use service::midi::{
    connect_input_device, connect_to_device, create_sysex_channel, list_midi_input_devices,
    list_midi_output_devices, send_cc,
};
use service::midi_mapper::match_midi_command;
use service::sysex_parser;
use model::{flow8::FLOW8Controller, message::InterfaceMessage, page::Page};

use iced::{
    widget::{column, scrollable},
    window, Element, Fill, Size, Subscription, Task, Theme,
};

fn open_url(url: &str) {
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open").arg(url).spawn();
    } else if cfg!(target_os = "windows") {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn();
    } else {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

fn clipboard_cmd() -> &'static str {
    if cfg!(target_os = "macos") {
        "pbcopy"
    } else if cfg!(target_os = "windows") {
        "clip"
    } else {
        "xclip"
    }
}

const APPLICATION_NAME: &str = "FLOW 8 MIDI Controller";
const PREFERRED_WIDTH: f32 = 1200.0;
const PREFERRED_HEIGHT: f32 = 700.0;
const MIN_WIDTH: f32 = 800.0;
const MIN_HEIGHT: f32 = 500.0;
static ICON: &[u8] = include_bytes!("../resources/icon.ico");

fn initial_window_size() -> Size {
    let (max_w, max_h) = screen_size_override().unwrap_or_else(available_screen_size);
    Size::new(PREFERRED_WIDTH.min(max_w), PREFERRED_HEIGHT.min(max_h))
}

fn screen_size_override() -> Option<(f32, f32)> {
    let val = std::env::var("FLOW8_SCREEN").ok()?;
    let (w, h) = val.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

#[cfg(target_os = "windows")]
fn available_screen_size() -> (f32, f32) {
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, pv: *mut Rect, flags: u32) -> i32;
        fn GetDpiForSystem() -> u32;
    }

    const SPI_GETWORKAREA: u32 = 0x0030;
    let mut rect = Rect { left: 0, top: 0, right: 0, bottom: 0 };

    let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect, 0) };
    if ok != 0 {
        let dpi = unsafe { GetDpiForSystem() };
        let scale = if dpi >= 96 { dpi as f32 / 96.0 } else { 1.0 };

        let w = (rect.right - rect.left) as f32 / scale;
        let h = (rect.bottom - rect.top) as f32 / scale;

        if w > 100.0 && h > 100.0 {
            return (w - 16.0, h - 40.0);
        }
    }

    (PREFERRED_WIDTH, PREFERRED_HEIGHT)
}

#[cfg(not(target_os = "windows"))]
fn available_screen_size() -> (f32, f32) {
    (PREFERRED_WIDTH, PREFERRED_HEIGHT)
}

fn boot() -> FLOW8Controller {
    logger::init();
    let mut controller = FLOW8Controller::new();

    // Load persisted user preferences before the rest of the app boots so the
    // initial theme and sync behavior match the previous session.
    match service::app_config::load() {
        Ok(config) => service::app_config::apply_to_controller(&config, &mut controller),
        Err(e) => log_warn!("[CONFIG] {}", e),
    }

    match service::tray::TrayManager::new() {
        Ok(tray_manager) => {
            controller.tray_manager = Some(tray_manager);
            log!("[TRAY] System tray manager initialized");
        }
        Err(e) => {
            log_warn!("[TRAY] Tray initialization failed: {}", e);
        }
    }

    controller.ble_available = ble::is_ble_available();
    if controller.ble_available {
        log!("[APP] BLE adapter detected");
        controller.ble_status = ble::BleStatus::Disconnected;
    } else {
        log!("[APP] No BLE adapter — auto-sync unavailable");
    }

    try_connect_flow8(&mut controller);
    controller
}

fn theme(state: &FLOW8Controller) -> Theme {
    state.theme.clone()
}

fn main() -> iced::Result {
    let image = load_from_memory(ICON).unwrap();
    let icon = window::icon::from_rgba(image.as_bytes().to_vec(), image.width(), image.height()).unwrap();

    iced::application(boot, update, view)
        .title(APPLICATION_NAME)
        .theme(theme)
        .subscription(subscription)
        .antialiasing(true)
        .window(window::Settings {
            size: initial_window_size(),
            min_size: Some(Size::new(MIN_WIDTH, MIN_HEIGHT)),
            resizable: true,
            icon: Some(icon),
            position: window::Position::Centered,
            exit_on_close_request: false,
            ..Default::default()
        })
        .run()
}

fn start_ble_connection(controller: &mut FLOW8Controller) {
    if !controller.ble_available {
        return;
    }
    if controller.ble_status == ble::BleStatus::Scanning
        || controller.ble_status == ble::BleStatus::Connecting
        || controller.ble_status == ble::BleStatus::Connected
    {
        return;
    }

    let (status_tx, status_rx) = std::sync::mpsc::channel();
    let (snapshot_tx, snapshot_rx) = std::sync::mpsc::channel();
    controller.ble_status_receiver = Some(status_rx);
    controller.snapshot_names_receiver = Some(snapshot_rx);
    controller.ble_status = ble::BleStatus::Scanning;

    let conn_arc = controller.ble_connection.clone();
    std::thread::spawn(move || match ble::connect_flow8_ble(status_tx, snapshot_tx) {
        Ok(conn) => {
            log!("[BLE] Connection established. Sending dump trigger...");
            let trigger_result = ble::send_dump_trigger(&conn);
            if let Ok(mut guard) = conn_arc.lock() {
                *guard = Some(conn);
            }
            match trigger_result {
                Ok(_) => log!("[BLE] Initial dump trigger sent"),
                Err(e) => log_error!("[BLE] Initial dump trigger failed: {}", e),
            }
        }
        Err(e) => {
            log_error!("[BLE] Connection failed: {}", e);
        }
    });
}

fn try_connect_flow8(controller: &mut FLOW8Controller) {
    let output_devices = list_midi_output_devices();
    let flow8_output = match output_devices.iter().find(|d| d.is_flow8) {
        Some(device) => device.clone(),
        None => {
            let msg = "FLOW 8 not found. Connect it via USB and try again.";
            log!("[APP] {}", msg);
            controller.connection_error = Some(msg.to_string());
            controller.current_page = Page::DeviceSelect;
            return;
        }
    };

    log!("[APP] Connecting to \"{}\" (port {})...", flow8_output.name, flow8_output.index);
    match connect_to_device(flow8_output.index) {
        Ok(conn) => {
            controller.midi_conn = Some(conn);
            controller.connected_device_name = Some(flow8_output.name);
            controller.connection_error = None;
            controller.current_page = Page::Mixer;

            let input_devices = list_midi_input_devices();
            if let Some(input_dev) = input_devices.iter().find(|d| d.is_flow8) {
                let (sysex_tx, sysex_rx) = create_sysex_channel();
                if let Ok(input_conn) = connect_input_device(input_dev.index, sysex_tx) {
                    controller.midi_input_conn = Some(input_conn);
                    controller.sysex_receiver = Some(sysex_rx);
                    log!("[APP] SysEx channel established");
                }
            }

            start_ble_connection(controller);
        }
        Err(e) => {
            log!("[APP] Connection failed: {}", e);
            controller.connection_error = Some(e);
            controller.current_page = Page::DeviceSelect;
        }
    }
}

fn subscription(_controller: &FLOW8Controller) -> Subscription<InterfaceMessage> {
    Subscription::batch([
        iced::time::every(std::time::Duration::from_millis(200)).map(|_| InterfaceMessage::Tick),
        window::close_requests().map(InterfaceMessage::WindowCloseRequested),
    ])
}

fn update(controller: &mut FLOW8Controller, message: InterfaceMessage) -> Task<InterfaceMessage> {
    match_midi_command(&message, &mut controller.midi_conn);
    update_interface(controller, message)
}

fn try_ble_dump_trigger(controller: &FLOW8Controller) -> Option<Result<(), String>> {
    let conn_arc = controller.ble_connection.clone();
    conn_arc
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(ble::send_dump_trigger))
}

fn persist_user_settings(controller: &FLOW8Controller) {
    // Persist settings immediately when they change. This keeps the behavior
    // simple and avoids losing UI preferences on crashes or forced exits.
    if let Err(e) = service::app_config::save_from_controller(controller) {
        log_warn!("[CONFIG] {}", e);
    }
}

fn view(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    if controller.current_page == Page::DeviceSelect {
        return view_device_select(controller);
    }

    let nav = view_nav_bar(controller);

    let page_content = match controller.current_page {
        Page::Mixer => view_mixer(controller),
        Page::Eq => view_eq(controller),
        Page::Sends => view_sends(controller),
        Page::Fx => view_fx(controller),
        Page::Snapshots => view_snapshots(controller),
        Page::Settings => view_settings(controller),
        Page::DeviceSelect => unreachable!(),
    };

    column![nav, scrollable(page_content).height(Fill)]
        .width(Fill)
        .height(Fill)
        .into()
}

const SNAPSHOT_RESYNC_DELAY_MS: u64 = 500;

fn update_interface(
    controller: &mut FLOW8Controller,
    message: InterfaceMessage,
) -> Task<InterfaceMessage> {
    let mut tasks: Vec<Task<InterfaceMessage>> = Vec::new();

    match message {
        InterfaceMessage::NavigateTo(page) => {
            controller.current_page = page;
        }
        InterfaceMessage::RetryConnection => {
            log!("[APP] Retrying connection...");
            controller.connection_error = None;
            try_connect_flow8(controller);
        }
        InterfaceMessage::Disconnect => {
            log!("[APP] Disconnecting BLE...");
            if let Ok(mut guard) = controller.ble_connection.lock() {
                if let Some(ref conn) = *guard {
                    ble::disconnect(conn);
                }
                *guard = None;
            }
            controller.ble_status = ble::BleStatus::Disconnected;
            controller.ble_status_receiver = None;
            controller.snapshot_names_receiver = None;
            controller.last_sync_time = None;
            controller.mark_all_unsynced();
        }

        InterfaceMessage::Mute(ch, _) => {
            let c = &mut controller.channels[ch as usize];
            c.is_muted = !c.is_muted;
        }
        InterfaceMessage::Solo(ch, _) => {
            let c = &mut controller.channels[ch as usize];
            c.is_soloed = !c.is_soloed;
        }
        InterfaceMessage::Gain(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.gain = value;
            c.channel_strip.gain_synced = true;
        }
        InterfaceMessage::Level(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.level = value;
            c.channel_strip.level_synced = true;
        }
        InterfaceMessage::Balance(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.balance = value;
            c.channel_strip.balance_synced = true;
        }
        InterfaceMessage::LowCut(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.low_cut = value;
            c.channel_strip.low_cut_synced = true;
        }
        InterfaceMessage::Compressor(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.compressor = value;
            c.channel_strip.compressor_synced = true;
        }
        InterfaceMessage::PhantomPower(ch) => {
            let now = std::time::Instant::now();
            let c = &mut controller.channels[ch as usize];
            let is_double = c
                .phantom_last_click
                .map(|prev| now.duration_since(prev).as_millis() < 500)
                .unwrap_or(false);
            if is_double {
                c.phantom_pwr.is_on = !c.phantom_pwr.is_on;
                c.phantom_last_click = None;
                if let Some(conn) = controller.midi_conn.as_mut() {
                    send_cc(conn, ch, 12, if c.phantom_pwr.is_on { 127 } else { 0 });
                }
            } else {
                c.phantom_last_click = Some(now);
            }
        }

        InterfaceMessage::EqLow(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.four_band_eq.low = value;
            c.four_band_eq.low_synced = true;
        }
        InterfaceMessage::EqLowMid(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.four_band_eq.low_mid = value;
            c.four_band_eq.low_mid_synced = true;
        }
        InterfaceMessage::EqHiMid(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.four_band_eq.hi_mid = value;
            c.four_band_eq.hi_mid_synced = true;
        }
        InterfaceMessage::EqHi(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.four_band_eq.hi = value;
            c.four_band_eq.hi_synced = true;
        }

        InterfaceMessage::SendMon1(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.mon1 = value;
            c.sends.mon1_synced = true;
        }
        InterfaceMessage::SendMon2(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.mon2 = value;
            c.sends.mon2_synced = true;
        }
        InterfaceMessage::SendFx1(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.fx1 = value;
            c.sends.fx1_synced = true;
        }
        InterfaceMessage::SendFx2(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.fx2 = value;
            c.sends.fx2_synced = true;
        }

        InterfaceMessage::BusLevel(bus_idx, _, value) => {
            let b = &mut controller.buses[bus_idx as usize];
            b.bus_strip.level = value;
            b.bus_strip.level_synced = true;
        }
        InterfaceMessage::BusBalance(bus_idx, _, value) => {
            let b = &mut controller.buses[bus_idx as usize];
            b.bus_strip.balance = value;
            b.bus_strip.balance_synced = true;
        }
        InterfaceMessage::BusLimiter(bus_idx, _, value) => {
            let b = &mut controller.buses[bus_idx as usize];
            b.bus_strip.limiter = value;
            b.bus_strip.limiter_synced = true;
        }
        InterfaceMessage::BusNineBandEq(bus_idx, _, band_index, value) => {
            let eq = &mut controller.buses[bus_idx as usize].nine_band_eq;
            match band_index {
                0 => eq.freq_62_hz = value,
                1 => eq.freq_125_hz = value,
                2 => eq.freq_250_hz = value,
                3 => eq.freq_500_hz = value,
                4 => eq.freq_1_khz = value,
                5 => eq.freq_2_khz = value,
                6 => eq.freq_4_khz = value,
                7 => eq.freq_8_khz = value,
                8 => eq.freq_16_khz = value,
                _ => {}
            }
            if (band_index as usize) < 9 {
                eq.bands_synced[band_index as usize] = true;
            }
        }

        InterfaceMessage::FxPreset(fx_id, value) => {
            let f = &mut controller.fx_slots[fx_id as usize];
            f.preset = value;
            f.preset_synced = true;
        }
        InterfaceMessage::FxParam1(fx_id, value) => {
            let f = &mut controller.fx_slots[fx_id as usize];
            f.param1 = value;
            f.param1_synced = true;
        }
        InterfaceMessage::FxParam2(fx_id, value) => {
            let f = &mut controller.fx_slots[fx_id as usize];
            f.param2 = value;
            f.param2_synced = true;
        }
        InterfaceMessage::FxMute => {
            controller.fx_muted = !controller.fx_muted;
            if let Some(conn) = controller.midi_conn.as_mut() {
                send_cc(conn, 15, 1, if controller.fx_muted { 127 } else { 0 });
            }
        }
        InterfaceMessage::TapTempo => {}

        InterfaceMessage::Tick => {
            if controller.main_window_id.is_none() {
                tasks.push(window::latest().map(InterfaceMessage::MainWindowIdResolved));
            }

            if let Some(ref tray) = controller.tray_manager {
                while let Some(tray_event) = tray.try_recv() {
                    match tray_event {
                        service::tray::TrayEvent::RestoreRequested => {
                            tray.hide_icon();
                            controller.window_hidden_to_tray = false;
                            service::tray::show_app_window();
                            log!("[TRAY] Restore requested");
                        }
                        service::tray::TrayEvent::ExitRequested => {
                            tray.shutdown();
                            controller.window_hidden_to_tray = false;
                            log!("[TRAY] Exit requested");
                            if let Some(id) = controller.main_window_id {
                                return window::close(id);
                            }
                        }
                    }
                }
            }

            controller.tick_counter = controller.tick_counter.wrapping_add(1);

            let pending: Vec<Vec<u8>> = controller
                .sysex_receiver
                .as_ref()
                .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
                .unwrap_or_default();

            for data in pending {
                log!("[SYSEX] Received data ({} bytes)", data.len());

                #[cfg(any(debug_assertions, feature = "dev-tools"))]
                if controller.calibration.is_running() {
                    controller.calibration.on_dump_received(data);
                    continue;
                }

                if let Some(dump) = sysex_parser::validate_sysex_dump(&data) {
                    sysex_parser::apply_dump_to_controller(&dump, controller);
                }
            }

            #[cfg(any(debug_assertions, feature = "dev-tools"))]
            if controller.calibration.is_running() {
                let action = controller.calibration.tick();
                if let Some((midi_ch, cc, value)) = action.send_cc {
                    if let Some(conn) = controller.midi_conn.as_mut() {
                        send_cc(conn, midi_ch, cc, value);
                    }
                }
                if let Some((midi_ch, program)) = action.send_pc {
                    if let Some(conn) = controller.midi_conn.as_mut() {
                        service::midi::send_program_change(conn, midi_ch, program);
                    }
                }
                if action.trigger_dump {
                    match try_ble_dump_trigger(controller) {
                        Some(Ok(_)) => {}
                        Some(Err(e)) => log_error!("[CALIB] Dump trigger failed: {}", e),
                        None => log_warn!("[CALIB] No BLE connection for dump trigger"),
                    }
                }
            }

            if let Some(ref ble_rx) = controller.ble_status_receiver {
                while let Ok(status) = ble_rx.try_recv() {
                    controller.ble_status = status;
                    log!("[BLE] Status: {}", status);
                }
            }

            if let Some(ref snap_rx) = controller.snapshot_names_receiver {
                while let Ok(names) = snap_rx.try_recv() {
                    log!("[BLE] Received {} snapshot names", names.len());
                    controller.snapshot_names = names;
                }
            }

            if let Some(pending_at) = controller.snapshot_resync_at {
                if pending_at.elapsed().as_millis() >= SNAPSHOT_RESYNC_DELAY_MS as u128 {
                    controller.snapshot_resync_at = None;
                    match try_ble_dump_trigger(controller) {
                        Some(Ok(_)) => {
                            controller.last_sync_time = Some(std::time::Instant::now());
                            log!("[APP] Post-snapshot resync triggered");
                        }
                        Some(Err(e)) => log_error!("[APP] Post-snapshot resync failed: {}", e),
                        None => log_warn!("[APP] No BLE connection for post-snapshot resync"),
                    }
                }
            }

            if controller.ble_status == ble::BleStatus::Connected {
                if let Some(interval_secs) = controller.sync_interval.as_secs() {
                    let should_sync = controller
                        .last_sync_time
                        .map(|t| t.elapsed().as_secs() >= interval_secs)
                        .unwrap_or(true);
                    if should_sync {
                        match try_ble_dump_trigger(controller) {
                            Some(Ok(_)) => {
                                controller.last_sync_time = Some(std::time::Instant::now());
                                log!("[APP] Auto-sync triggered");
                            }
                            Some(Err(e)) => log_error!("[APP] Auto-sync dump failed: {}", e),
                            None => {}
                        }
                    }
                }
            }
        }
        InterfaceMessage::WindowCloseRequested(id) => {
            if controller.close_to_tray_on_close {
                if let Some(ref tray) = controller.tray_manager {
                    tray.show_icon();
                    service::tray::hide_app_window();
                    controller.window_hidden_to_tray = true;
                    log!("[TRAY] Window hidden to system tray");
                    return Task::none();
                }
            }

            if let Some(ref tray) = controller.tray_manager {
                tray.shutdown();
            }
            return window::close(id);
        }
        InterfaceMessage::MainWindowIdResolved(id) => {
            if controller.main_window_id.is_none() {
                controller.main_window_id = id;
            }
        }
        InterfaceMessage::TrayEvent(_) => {}

        InterfaceMessage::BleConnect => {
            let now = std::time::Instant::now();
            let is_double = controller
                .ble_last_click
                .map(|prev| now.duration_since(prev).as_millis() < 500)
                .unwrap_or(false);
            if is_double {
                controller.ble_last_click = None;
                start_ble_connection(controller);
            } else {
                controller.ble_last_click = Some(now);
            }
        }

        InterfaceMessage::BleRequestDump => {
            controller.sync_last_click = Some(std::time::Instant::now());
            match try_ble_dump_trigger(controller) {
                Some(Ok(_)) => {
                    controller.last_sync_time = Some(std::time::Instant::now());
                    log!("[APP] Dump trigger sent via BLE");
                }
                Some(Err(e)) => log_error!("[APP] Dump trigger failed: {}", e),
                None => log_warn!("[APP] No BLE connection for dump trigger"),
            }
        }

        InterfaceMessage::LoadSnapshot(n) => {
            log!("[APP] Loading snapshot {}", n + 1);
            controller.mark_all_unsynced();
            controller.snapshot_resync_at = Some(std::time::Instant::now());
        }
        InterfaceMessage::ResetMixer => {
            log!("[APP] Resetting mixer to default");
            controller.mark_all_unsynced();
            controller.snapshot_resync_at = Some(std::time::Instant::now());
        }

        InterfaceMessage::ThemeChanged(theme) => {
            controller.theme = theme;
            persist_user_settings(controller);
        }
        InterfaceMessage::SyncIntervalChanged(interval) => {
            controller.sync_interval = interval;
            log!("[APP] Sync interval changed to {}", interval);
            persist_user_settings(controller);
        }
        InterfaceMessage::CloseToTrayChanged(enabled) => {
            controller.close_to_tray_on_close = enabled;
            log!(
                "[APP] Close button behavior changed: {}",
                if enabled { "minimize to tray" } else { "close app" }
            );
            persist_user_settings(controller);
        }
        InterfaceMessage::OpenManual => {
            open_url("https://github.com/abelroes/flow-8-midi/blob/main/docs/MANUAL.md");
        }
        InterfaceMessage::OpenRepository => {
            open_url("https://github.com/abelroes/flow-8-midi");
        }
        InterfaceMessage::OpenDonation => {
            open_url("https://buymeacoffee.com/abelroes");
        }
        InterfaceMessage::CopyDebugLog => {
            let export = logger::export_log();
            if let Ok(mut child) = std::process::Command::new(clipboard_cmd())
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(ref mut stdin) = child.stdin {
                    use std::io::Write;
                    let _ = stdin.write_all(export.as_bytes());
                }
                let _ = child.wait();
                log!("[APP] Debug log copied to clipboard");
            } else {
                log_warn!("[APP] Could not access clipboard");
            }
        }
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        InterfaceMessage::CopyHexDump => {
            if let Some(ref dump) = controller.last_sysex_dump {
                let hex = sysex_parser::format_hex_dump(dump);
                if let Ok(mut child) = std::process::Command::new(clipboard_cmd())
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    if let Some(ref mut stdin) = child.stdin {
                        use std::io::Write;
                        let _ = stdin.write_all(hex.as_bytes());
                    }
                    let _ = child.wait();
                    log!("[APP] Hex dump copied to clipboard ({} bytes)", dump.len());
                }
            } else {
                log_warn!("[APP] No SysEx dump available to copy");
            }
        }
        InterfaceMessage::SaveDebugLog => {
            let export = logger::export_log();
            let filename = format!(
                "flow8-debug-{}.log",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            match std::fs::write(&filename, &export) {
                Ok(_) => log!("[APP] Debug log saved to {}", filename),
                Err(e) => log_error!("[APP] Failed to save debug log: {}", e),
            }
        }
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        InterfaceMessage::CalibrateStart => {
            if controller.calibration.is_running() {
                log_warn!("[APP] Calibration already in progress");
                return;
            }
            if controller.midi_conn.is_none() {
                log_warn!("[APP] Cannot calibrate: no MIDI connection");
                return;
            }
            let has_ble = controller
                .ble_connection
                .lock()
                .ok()
                .map(|g| g.is_some())
                .unwrap_or(false);
            if !has_ble {
                log_warn!("[APP] Cannot calibrate: no BLE connection (needed to trigger dumps)");
                return;
            }
            log!("[APP] Starting SysEx calibration...");
            controller.calibration.start();
        }
        #[cfg(any(debug_assertions, feature = "dev-tools"))]
        InterfaceMessage::DigestRun => {
            log!("[APP] Running file-based digest...");
            match service::sysex_calibration::run_file_based_digest() {
                Ok(summary) => log!("[APP] Digest complete: {}", summary),
                Err(e) => log_warn!("[APP] Digest failed: {}", e),
            }
        }
    }

    Task::batch(tasks)
}
