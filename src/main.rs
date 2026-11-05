#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod logger;
mod model;
mod platform;
pub mod service;
mod view;

use image::load_from_memory;
use model::{flow8::FLOW8Controller, message::InterfaceMessage, page::Page};
use service::ble;
use service::midi::{
    connect_input_device, connect_to_device, create_sysex_channel, list_midi_input_devices,
    list_midi_output_devices, send_cc,
};
use service::midi_mapper::match_midi_command;
use service::sysex_parser;
use view::{
    connection_page::view_connection, device_select_page::view_device_select, eq_page::view_eq,
    fx_page::view_fx, home_page::view_home, mixer_page::view_mixer, nav_bar::view_nav_bar,
    sends_page::view_sends, settings_page::view_settings, snapshots_page::view_snapshots,
    stage_page::view_stage, widgets::titlebar,
};

use iced::{
    widget::{column, scrollable},
    window, Element, Fill, Size, Subscription, Theme,
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
    let mut rect = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

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

    let instance_guard = match platform::single_instance::ensure_single_instance() {
        Ok(guard) => Some(guard),
        Err(e) => {
            eprintln!("[APP] {}", e);
            std::process::exit(1);
        }
    };

    let mut controller = FLOW8Controller::new();
    controller._instance_guard = instance_guard;

    if let Err(e) = platform::tray::create_tray_menu() {
        log_warn!("[APP] Tray init skipped: {}", e);
    }

    controller.ble_available = ble::is_ble_available();
    if controller.ble_available {
        log!("[APP] BLE adapter detected");
        controller.ble_status = ble::BleStatus::Disconnected;
    } else {
        log!("[APP] No BLE adapter — use USB MIDI from Connection page");
    }

    controller
}

fn theme(state: &FLOW8Controller) -> Theme {
    match state.theme {
        Theme::Light => Theme::Light,
        _ => view::theme::flow8_theme(),
    }
}

fn main() -> iced::Result {
    let image = load_from_memory(ICON).unwrap();
    let icon =
        window::icon::from_rgba(image.as_bytes().to_vec(), image.width(), image.height()).unwrap();

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
            ..Default::default()
        })
        .run()
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

    log!(
        "[APP] Connecting to \"{}\" (port {})...",
        flow8_output.name,
        flow8_output.index
    );
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

        }
        Err(e) => {
            log!("[APP] Connection failed: {}", e);
            controller.connection_error = Some(e);
            controller.current_page = Page::Connection;
        }
    }
}

fn subscription(_controller: &FLOW8Controller) -> Subscription<InterfaceMessage> {
    Subscription::batch([
        iced::time::every(std::time::Duration::from_millis(200)).map(|_| InterfaceMessage::Tick),
        service::ble_subscription::ble_subscription().map(InterfaceMessage::BleSubscription),
    ])
}

fn update(controller: &mut FLOW8Controller, message: InterfaceMessage) {
    let usb_mode =
        controller.midi_conn.is_some() && !controller.is_ble_active();
    if usb_mode {
        match_midi_command(&message, &mut controller.midi_conn);
    }
    update_interface(controller, message);
}

fn send_ble(controller: &FLOW8Controller, cmd: service::ble_protocol::BleCommand) {
    if let Some(ref tx) = controller.ble_cmd_tx {
        if let Err(e) = tx.try_send(cmd) {
            log_warn!("[BLE] Command send failed: {}", e);
        }
    }
}

fn try_ble_dump_trigger(controller: &FLOW8Controller) -> Option<Result<(), String>> {
    let conn_arc = controller.ble_connection.clone();
    conn_arc
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(ble::send_dump_trigger))
}

fn view(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    use iced::widget::container;
    use view::theme;

    match controller.current_page {
        Page::Connection => {
            return container(view_connection(controller))
                .width(Fill)
                .height(Fill)
                .style(theme::app_box)
                .into();
        }
        Page::DeviceSelect => {
            return container(view_device_select(controller))
                .width(Fill)
                .height(Fill)
                .style(theme::app_box)
                .into();
        }
        _ => {}
    }

    let chrome = column![titlebar(), view_nav_bar(controller)].width(Fill);

    let page_content = match controller.current_page {
        Page::Home => view_home(controller),
        Page::Mixer => view_mixer(controller),
        Page::Stage => view_stage(controller),
        Page::Eq => view_eq(controller),
        Page::Sends => view_sends(controller),
        Page::Fx => view_fx(controller),
        Page::Snapshots => view_snapshots(controller),
        Page::Setup | Page::Settings => view_settings(controller),
        Page::Connection | Page::DeviceSelect => unreachable!(),
    };

    container(
        column![chrome, scrollable(page_content).height(Fill)]
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(theme::app_box)
    .into()
}

const SNAPSHOT_RESYNC_DELAY_MS: u64 = 500;

fn update_interface(controller: &mut FLOW8Controller, message: InterfaceMessage) {
    match message {
        InterfaceMessage::NavigateTo(page) => {
            match page {
                Page::Setup => {
                    controller.setup_tab = 0;
                    controller.current_page = Page::Setup;
                }
                Page::Snapshots => {
                    controller.setup_tab = 2;
                    controller.current_page = Page::Snapshots;
                }
                other => controller.current_page = other,
            }
        }
        InterfaceMessage::RetryConnection => {
            log!("[APP] Retrying connection...");
            controller.connection_error = None;
            try_connect_flow8(controller);
        }
        InterfaceMessage::Disconnect => {
            log!("[APP] Disconnecting...");
            if let Some(ref tx) = controller.ble_control_tx {
                let _ = tx.try_send(service::ble_session::SessionControl::Cancel);
            }
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
            controller.session_state = service::ble_session::SessionState::Disconnected;
            controller.meter.reset();
            controller.midi_conn = None;
            controller.midi_input_conn = None;
            controller.connected_device_name = None;
            controller.sysex_receiver = None;
            controller.mark_all_unsynced();
            controller.current_page = Page::Connection;
        }

        InterfaceMessage::Mute(ch, _) => {
            let c = &mut controller.channels[ch as usize];
            c.is_muted = !c.is_muted;
            let muted = c.is_muted;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMute {
                        id: ch,
                        value: muted,
                    },
                );
            }
        }
        InterfaceMessage::Solo(ch, _) => {
            let c = &mut controller.channels[ch as usize];
            c.is_soloed = !c.is_soloed;
            let soloed = c.is_soloed;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelSolo {
                        id: ch,
                        value: soloed,
                    },
                );
            }
        }
        InterfaceMessage::Gain(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.gain = value;
            c.channel_strip.gain_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelGain {
                        id: ch,
                        db: service::mixer_state::gain_cc_to_db(value),
                    },
                );
            }
        }
        InterfaceMessage::Level(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.level = value;
            c.channel_strip.level_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMaster {
                        src: ch,
                        dst: service::mixer_state::BLE_MAIN,
                        db: service::mixer_state::level_cc_to_db(value),
                    },
                );
            }
        }
        InterfaceMessage::Balance(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.channel_strip.balance = value;
            c.channel_strip.balance_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelPan {
                        id: ch,
                        value: service::mixer_state::cc_to_balance(value),
                    },
                );
            }
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
            let is_double = controller.channels[ch as usize]
                .phantom_last_click
                .map(|prev| now.duration_since(prev).as_millis() < 500)
                .unwrap_or(false);
            if is_double {
                let on = {
                    let c = &mut controller.channels[ch as usize];
                    c.phantom_pwr.is_on = !c.phantom_pwr.is_on;
                    c.phantom_last_click = None;
                    c.phantom_pwr.is_on
                };
                if controller.is_ble_active() {
                    send_ble(
                        controller,
                        service::ble_protocol::BleCommand::ChannelPhantomPower {
                            id: ch,
                            value: on,
                        },
                    );
                } else if let Some(conn) = controller.midi_conn.as_mut() {
                    send_cc(conn, ch, 12, if on { 127 } else { 0 });
                }
            } else {
                controller.channels[ch as usize].phantom_last_click = Some(now);
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
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMaster {
                        src: ch,
                        dst: service::mixer_state::BLE_MON1,
                        db: service::mixer_state::level_cc_to_db(value),
                    },
                );
            }
        }
        InterfaceMessage::SendMon2(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.mon2 = value;
            c.sends.mon2_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMaster {
                        src: ch,
                        dst: service::mixer_state::BLE_MON2,
                        db: service::mixer_state::level_cc_to_db(value),
                    },
                );
            }
        }
        InterfaceMessage::SendFx1(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.fx1 = value;
            c.sends.fx1_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMaster {
                        src: ch,
                        dst: service::mixer_state::BLE_FX1,
                        db: service::mixer_state::level_cc_to_db(value),
                    },
                );
            }
        }
        InterfaceMessage::SendFx2(ch, value) => {
            let c = &mut controller.channels[ch as usize];
            c.sends.fx2 = value;
            c.sends.fx2_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMaster {
                        src: ch,
                        dst: service::mixer_state::BLE_FX2,
                        db: service::mixer_state::level_cc_to_db(value),
                    },
                );
            }
        }

        InterfaceMessage::BusLevel(bus_idx, bus_id, value) => {
            let b = &mut controller.buses[bus_idx as usize];
            b.bus_strip.level = value;
            b.bus_strip.level_synced = true;
            if controller.is_ble_active() {
                let ble_id = service::mixer_state::ui_bus_id_to_ble(bus_id);
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelMaster {
                        src: ble_id,
                        dst: ble_id,
                        db: service::mixer_state::level_cc_to_db(value),
                    },
                );
            }
        }
        InterfaceMessage::BusBalance(bus_idx, bus_id, value) => {
            let b = &mut controller.buses[bus_idx as usize];
            b.bus_strip.balance = value;
            b.bus_strip.balance_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelPan {
                        id: service::mixer_state::ui_bus_id_to_ble(bus_id),
                        value: service::mixer_state::cc_to_balance(value),
                    },
                );
            }
        }
        InterfaceMessage::BusLimiter(bus_idx, bus_id, value) => {
            let b = &mut controller.buses[bus_idx as usize];
            b.bus_strip.limiter = value;
            b.bus_strip.limiter_synced = true;
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::ChannelLimiter {
                        id: service::mixer_state::ui_bus_id_to_ble(bus_id),
                        db: service::mixer_state::limiter_cc_to_db(value),
                    },
                );
            }
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
            controller.tick_counter = controller.tick_counter.wrapping_add(1);
            controller.meter.tick();

            let pending: Vec<Vec<u8>> = controller
                .sysex_receiver
                .as_ref()
                .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
                .unwrap_or_default();

            for data in pending {
                log!("[SYSEX] Received data ({} bytes)", data.len());

                if let Some(dump) = sysex_parser::validate_sysex_dump(&data) {
                    sysex_parser::apply_dump_to_controller(&dump, controller);
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

        InterfaceMessage::BleConnect => {
            log!("[APP] BLE reconnect via session (subscription)");
            controller.connection_error = None;
            if controller.session_state == service::ble_session::SessionState::Active {
                controller.current_page = Page::Home;
            } else {
                controller.current_page = Page::Connection;
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
            if controller.is_ble_active() {
                send_ble(
                    controller,
                    service::ble_protocol::BleCommand::SnapshotLoad { slot: n },
                );
                send_ble(controller, service::ble_protocol::BleCommand::GetMixerState);
            } else {
                controller.snapshot_resync_at = Some(std::time::Instant::now());
            }
        }
        InterfaceMessage::ResetMixer => {
            log!("[APP] Resetting mixer to default");
            controller.mark_all_unsynced();
            controller.meter.reset();
            controller.snapshot_resync_at = Some(std::time::Instant::now());
        }

        InterfaceMessage::ThemeChanged(theme) => {
            controller.theme = theme;
        }
        InterfaceMessage::SyncIntervalChanged(interval) => {
            controller.sync_interval = interval;
            log!("[APP] Sync interval changed to {}", interval);
        }
        InterfaceMessage::CloseToTrayChanged(enabled) => {
            controller.tray_config.close_to_tray = enabled;
            log!("[APP] Close-to-tray: {}", enabled);
            if enabled {
                if let Err(e) = platform::tray::create_tray_menu() {
                    log_warn!("[APP] Tray init failed: {}", e);
                }
            }
        }
        InterfaceMessage::SelectBus(bus) => {
            controller.selected_bus = bus;
            log!("[APP] Bus selector: 0x{:02X}", bus);
        }
        InterfaceMessage::SelectSetupTab(tab) => {
            controller.setup_tab = tab;
            controller.current_page = match tab {
                2 => Page::Snapshots,
                _ => Page::Setup,
            };
        }
        InterfaceMessage::SelectChannel(ch) => {
            controller.selected_channel = ch;
            controller.detail_is_bus = false;
        }
        InterfaceMessage::SelectFxSlot(slot) => {
            controller.selected_fx = slot.min(1);
        }
        InterfaceMessage::EditChannel(ch) => {
            controller.selected_channel = ch;
            controller.detail_is_bus = false;
            controller.current_page = Page::Eq;
        }
        InterfaceMessage::EditBus(bus_idx) => {
            controller.selected_bus_edit = bus_idx;
            controller.detail_is_bus = true;
            controller.current_page = Page::Eq;
        }
        InterfaceMessage::ToggleAppPref(idx) => {
            if let Some(flag) = controller.app_prefs.get_mut(idx as usize) {
                *flag = !*flag;
            }
        }
        InterfaceMessage::ToggleRouting(idx) => {
            if let Some(flag) = controller.routing_flags.get_mut(idx as usize) {
                *flag = !*flag;
            }
        }
        InterfaceMessage::ToggleSnapshotScope(idx) => {
            if let Some(flag) = controller.snapshot_scope.get_mut(idx as usize) {
                *flag = !*flag;
            }
        }
        InterfaceMessage::DismissFirmwareNotice => {
            controller.firmware_notice_dismissed = true;
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
        #[cfg(debug_assertions)]
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

        InterfaceMessage::ConnectBle => {
            log!("[APP] Connect BLE requested");
            controller.connection_error = None;
            if controller.session_state == service::ble_session::SessionState::Active {
                controller.current_page = Page::Home;
            } else if let Some(ref tx) = controller.ble_control_tx {
                let _ = tx.try_send(service::ble_session::SessionControl::Connect);
                #[cfg(target_os = "windows")]
                {
                    controller.session_state = service::ble_session::SessionState::Preparing;
                }
                #[cfg(not(target_os = "windows"))]
                {
                    controller.session_state = service::ble_session::SessionState::Scanning;
                }
            } else {
                controller.connection_error =
                    Some("Bluetooth session not ready yet — try again in a moment.".into());
            }
        }
        InterfaceMessage::ConnectUsb => {
            log!("[APP] Connect USB requested");
            if let Some(ref tx) = controller.ble_control_tx {
                let _ = tx.try_send(service::ble_session::SessionControl::Cancel);
            }
            try_connect_flow8(controller);
        }
        InterfaceMessage::CancelConnection => {
            log!("[APP] Connection cancelled");
            if let Some(ref tx) = controller.ble_control_tx {
                let _ = tx.try_send(service::ble_session::SessionControl::Cancel);
            }
            controller.connection_error = Some("Connection cancelled".to_string());
            controller.session_state = service::ble_session::SessionState::Disconnected;
            controller.ble_status = ble::BleStatus::Disconnected;
        }
        InterfaceMessage::BleSubscription(event) => match event {
            service::ble_subscription::BleSubscriptionEvent::Ready {
                command_tx,
                control_tx,
            } => {
                log!("[BLE] Session command channel ready");
                controller.ble_cmd_tx = Some(command_tx);
                controller.ble_control_tx = Some(control_tx);
            }
            service::ble_subscription::BleSubscriptionEvent::StateChanged(state) => {
                let was_active =
                    controller.session_state == service::ble_session::SessionState::Active;
                controller.session_state = state;
                log!("[BLE] Session state: {}", state);
                if state == service::ble_session::SessionState::Active && !was_active {
                    controller.connection_error = None;
                    controller.ble_status = ble::BleStatus::Connected;
                    if matches!(
                        controller.current_page,
                        Page::Connection | Page::DeviceSelect
                    ) {
                        controller.current_page = Page::Home;
                    }
                }
                if state == service::ble_session::SessionState::PairingRequired {
                    controller.current_page = Page::Connection;
                }
            }
            service::ble_subscription::BleSubscriptionEvent::Event(ble_event) => {
                handle_ble_event(controller, ble_event);
            }
            service::ble_subscription::BleSubscriptionEvent::Disconnected(reason) => {
                log!("[BLE] Disconnected: {}", reason);
                controller.session_state = service::ble_session::SessionState::Disconnected;
                controller.ble_status = ble::BleStatus::Disconnected;
                controller.connection_error = Some(reason);
                controller.meter.reset();
                if !matches!(
                    controller.current_page,
                    Page::Connection | Page::DeviceSelect
                ) {
                    controller.current_page = Page::Connection;
                }
            }
        },
    }
}

fn handle_ble_event(controller: &mut FLOW8Controller, event: service::ble_protocol::BleEvent) {
    use service::ble_protocol::BleEvent;
    match event {
        BleEvent::MeterUpdate(data) => {
            controller.meter.apply_update(&data.levels, data.gain_reduction);
        }
        BleEvent::MixerState(payload) => {
            match service::mixer_state::decode_mixer_state(&payload) {
                Ok(data) => {
                    service::mixer_state::apply_mixer_state(&data, controller);
                    log!(
                        "[BLE] MixerState applied ({} inputs, {} outputs)",
                        data.inputs.len(),
                        data.outputs.len()
                    );
                }
                Err(e) => log_error!("[BLE] MixerState decode failed: {}", e),
            }
        }
        BleEvent::HandshakeHost(data) => {
            controller.protocol_version = Some(data.protocol_version);
            controller.firmware_version = Some(format!("build {}", data.firmware_build));
        }
        BleEvent::SnapshotList(names) => {
            controller.snapshot_names = names;
        }
        BleEvent::ChannelConnection(data) => {
            if let Some(ch) = controller.channels.iter_mut().find(|c| c.id == data.id) {
                if data.sub_channel == 0 {
                    ch.connected_l = data.connected;
                } else {
                    ch.connected_r = data.connected;
                }
            }
        }
        BleEvent::PairingRequired => {
            controller.session_state = service::ble_session::SessionState::PairingRequired;
            controller.current_page = Page::Connection;
        }
        BleEvent::Disconnected(reason) => {
            log!("[BLE] Event disconnect: {}", reason);
            controller.connection_error = Some(reason);
            controller.meter.reset();
        }
        other => {
            log_debug!("[BLE] Event: {:?}", other);
        }
    }
}
