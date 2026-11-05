use crate::logger;
use crate::model::{
    flow8::{FLOW8Controller, SyncInterval},
    message::InterfaceMessage,
};
#[cfg(debug_assertions)]
use crate::service::sysex_parser;
use crate::service::ble_session::SessionState;
use crate::view::setup_nav::{card_section, setup_header};
use crate::view::theme::{self, PRIMARY, SECONDARY};
use crate::view::widgets::on_off_toggle;
use iced::{
    widget::{button, column, container, pick_list, row, scrollable, text, Space},
    Center, Element, Fill, Length, Theme,
};

const VISIBLE_ENTRIES: usize = 200;
#[cfg(debug_assertions)]
const HEX_PREVIEW_BYTES: usize = 256;

const APP_PREF_LABELS: [&str; 4] = [
    "Show dB values",
    "Finer sliders",
    "Mute buttons visible",
    "EQ full parametric",
];

const ROUTING_FX: [(&str, u8); 6] = [
    ("FX1 → Main", 0),
    ("FX2 → Main", 1),
    ("FX1 → Mon1", 2),
    ("FX2 → Mon1", 3),
    ("FX1 → Mon2", 4),
    ("FX2 → Mon2", 5),
];

const ROUTING_PHONES: [(&str, u8); 4] = [
    ("Phones source = Main", 6),
    ("Phones Pre/Post", 7),
    ("Monitor Pre/Post", 8),
    ("BT/USB play → phones only", 9),
];

const ROUTING_USB: [(&str, u8); 5] = [
    ("USB 1/2 → Ch 5/6", 10),
    ("USB 3/4 → Ch 7/8", 11),
    ("Record vs Stream", 12),
    ("App link layers follow", 13),
    ("Footswitch alt mode", 14),
];

pub fn view_settings(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let header = setup_header(controller.setup_tab);

    let body: Element<'_, InterfaceMessage> = match controller.setup_tab {
        1 => build_routing(controller),
        3 => build_info(controller),
        _ => column![
            card_section("Appearance & Sync", build_appearance_body(controller)),
            Space::new().height(12),
            card_section("App Preferences", build_app_prefs(controller)),
        ]
        .padding([0, 10])
        .into(),
    };

    let content = column![header, body].width(Fill);

    #[cfg(debug_assertions)]
    let content = {
        let mut c = content;
        if let Some(ref dump) = controller.last_sysex_dump {
            c = c.push(Space::new().height(16));
            c = c.push(build_hex_viewer(dump));
        }
        c
    };

    container(scrollable(content).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(theme::app_box)
        .into()
}

fn build_appearance_body(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let theme_row = row![
        text("Theme").size(12).color(SECONDARY),
        Space::new().width(Fill),
        pick_list(
            Theme::ALL,
            Some(&controller.theme),
            InterfaceMessage::ThemeChanged
        )
        .text_size(12)
        .padding([4, 8]),
    ]
    .align_y(Center);

    let control_row = row![
        text("Control").size(12).color(SECONDARY),
        Space::new().width(Fill),
        container(text("Bluetooth (BLE)").size(12).color(PRIMARY))
            .padding([6, 12])
            .style(theme::elevated_box),
    ]
    .align_y(Center);

    let sync_row = row![
        text("Sync interval").size(12).color(SECONDARY),
        Space::new().width(Fill),
        pick_list(
            SyncInterval::ALL,
            Some(controller.sync_interval),
            InterfaceMessage::SyncIntervalChanged,
        )
        .text_size(12)
        .padding([4, 8]),
        Space::new().width(8),
        button(text("Sync Now").size(11).center())
            .padding([6, 12])
            .on_press(InterfaceMessage::BleRequestDump)
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_y(Center);

    column![
        theme_row,
        Space::new().height(10),
        control_row,
        Space::new().height(10),
        sync_row,
        Space::new().height(8),
        text("Pull mixer state if the desk and the app disagree.")
            .size(11)
            .color(SECONDARY),
    ]
    .into()
}

fn build_app_prefs(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let mut col = column![];
    for (i, label) in APP_PREF_LABELS.iter().enumerate() {
        if i > 0 {
            col = col.push(Space::new().height(8));
        }
        col = col.push(pref_row(
            label,
            controller.app_prefs[i],
            InterfaceMessage::ToggleAppPref(i as u8),
        ));
    }
    col.into()
}

fn pref_row(
    label: &'static str,
    on: bool,
    msg: InterfaceMessage,
) -> Element<'static, InterfaceMessage> {
    row![
        text(label).size(12).color(PRIMARY),
        Space::new().width(Fill),
        on_off_toggle(on, msg),
    ]
    .align_y(Center)
    .into()
}

fn build_routing(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let fx_left = routing_group(
        "FX Returns",
        &ROUTING_FX[..3],
        controller,
    );
    let fx_right = routing_group("", &ROUTING_FX[3..], controller);
    let phones = routing_group("Phones / Monitor", &ROUTING_PHONES, controller);
    let usb = routing_group("USB / Record", &ROUTING_USB, controller);

    column![
        row![fx_left, Space::new().width(12), fx_right].width(Fill),
        Space::new().height(12),
        row![phones, Space::new().width(12), usb].width(Fill),
    ]
    .padding([0, 10])
    .into()
}

fn routing_group<'a>(
    title: &'static str,
    items: &'a [(&'static str, u8)],
    controller: &'a FLOW8Controller,
) -> Element<'a, InterfaceMessage> {
    let mut body = column![];
    for (i, (label, idx)) in items.iter().enumerate() {
        if i > 0 {
            body = body.push(Space::new().height(8));
        }
        let on = controller
            .routing_flags
            .get(*idx as usize)
            .copied()
            .unwrap_or(false);
        body = body.push(pref_row(label, on, InterfaceMessage::ToggleRouting(*idx)));
    }

    if title.is_empty() {
        container(body.padding(16))
            .width(Fill)
            .style(theme::surface_box)
            .into()
    } else {
        card_section(title, body.into())
    }
}

fn build_info(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let transport = if controller.is_ble_active() {
        "Bluetooth (BLE)"
    } else if controller.midi_conn.is_some() {
        "USB MIDI"
    } else {
        "—"
    };
    let ble_label = match controller.session_state {
        SessionState::Active => "Connected",
        SessionState::Disconnected => "Disconnected",
        _ => "Connecting…",
    };
    let device = controller
        .connected_device_name
        .as_deref()
        .unwrap_or("—");

    let about = column![
        text("FLOW 8 MIDI Controller").size(12).color(PRIMARY),
        Space::new().height(4),
        text("Unofficial desktop controller for Behringer FLOW 8.")
            .size(11)
            .color(SECONDARY),
        Space::new().height(10),
        row![
            button(text("GitHub").size(12).center())
                .padding([8, 14])
                .on_press(InterfaceMessage::OpenRepository)
                .style(theme::btn(theme::BtnKind::Secondary)),
            Space::new().width(8),
            button(text("Support").size(12).center())
                .padding([8, 14])
                .on_press(InterfaceMessage::OpenDonation)
                .style(theme::btn(theme::BtnKind::Secondary)),
        ],
    ];

    let session = column![
        text(format!("Mixer: FLOW 8")).size(12).color(PRIMARY),
        text(format!("Device: {}", device))
            .size(12)
            .color(PRIMARY),
        text(format!("Transport: {}", transport))
            .size(12)
            .color(PRIMARY),
        text(format!("BLE: {}", ble_label))
            .size(12)
            .color(PRIMARY),
        text("Mode: Live").size(12).color(PRIMARY),
        Space::new().height(12),
        build_log_section(controller),
    ];

    column![
        card_section("About", about.into()),
        Space::new().height(12),
        card_section("Manual", build_manual_section()),
        Space::new().height(12),
        card_section("Session", session.into()),
    ]
    .padding([0, 10])
    .into()
}

fn build_manual_section() -> Element<'static, InterfaceMessage> {
    column![
        text("Read the user manual for setup and usage instructions.")
            .size(12)
            .color(SECONDARY),
        Space::new().height(8),
        button(text("Open Manual").size(12).center())
            .on_press(InterfaceMessage::OpenManual)
            .padding([8, 14])
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .into()
}

fn build_log_section(_controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let entries = logger::get_recent_entries(VISIBLE_ENTRIES);
    let count = logger::entry_count();

    let mut log_col = iced::widget::Column::new().spacing(1);
    if count > VISIBLE_ENTRIES {
        log_col = log_col.push(
            text(format!(
                "... ({} earlier entries omitted)",
                count - VISIBLE_ENTRIES
            ))
            .size(10)
            .color(SECONDARY),
        );
    }
    for entry in &entries {
        let entry_text = text(format!("{}", entry)).size(11);
        let styled = match entry.level {
            logger::LogLevel::Error => entry_text.color(theme::ERROR),
            logger::LogLevel::Warn => entry_text.color(theme::WARNING),
            _ => entry_text.color(PRIMARY),
        };
        log_col = log_col.push(styled);
    }
    if entries.is_empty() {
        log_col = log_col.push(text("No log entries yet.").size(11).color(SECONDARY));
    }

    let actions = row![
        button(text("Copy to Clipboard").size(12).center())
            .on_press(InterfaceMessage::CopyDebugLog)
            .padding([6, 14])
            .style(theme::btn(theme::BtnKind::Secondary)),
        button(text("Save as File").size(12).center())
            .on_press(InterfaceMessage::SaveDebugLog)
            .padding([6, 14])
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .spacing(8);

    column![
        text("Debug Log").size(12).color(SECONDARY),
        Space::new().height(6),
        container(scrollable(log_col).height(Length::Fixed(160.0)))
            .style(theme::elevated_box)
            .width(Fill)
            .padding(8),
        Space::new().height(8),
        actions,
    ]
    .into()
}

#[cfg(debug_assertions)]
fn build_hex_viewer(dump: &[u8]) -> Element<'_, InterfaceMessage> {
    let preview_len = HEX_PREVIEW_BYTES.min(dump.len());
    let hex_text = sysex_parser::format_hex_dump(&dump[..preview_len]);
    let suffix = if dump.len() > preview_len {
        format!("\n... ({} more bytes)", dump.len() - preview_len)
    } else {
        String::new()
    };

    let hex_header = row![
        text(format!("Last SysEx Dump ({} bytes)", dump.len())).size(14),
        Space::new().width(Fill),
        button(text("Copy Full Hex Dump").size(11))
            .on_press(InterfaceMessage::CopyHexDump)
            .padding([4, 10])
            .style(button::secondary),
    ]
    .align_y(Center)
    .spacing(8);

    let mono_color = iced::Color::from_rgb(0.6, 0.8, 0.6);

    container(
        column![
            hex_header,
            Space::new().height(4),
            container(
                scrollable(
                    text(format!("{}{}", hex_text, suffix))
                        .size(10)
                        .color(mono_color)
                )
                .height(Length::Fixed(160.0))
            )
            .style(container::rounded_box)
            .width(Fill)
            .padding(8),
        ]
        .padding([12, 14])
        .spacing(2)
        .width(Fill),
    )
    .style(container::rounded_box)
    .width(Fill)
    .into()
}
