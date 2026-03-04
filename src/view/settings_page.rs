use crate::logger;
use crate::model::{
    flow8::{FLOW8Controller, SyncInterval},
    message::InterfaceMessage,
};
#[cfg(any(debug_assertions, feature = "dev-tools"))]
use crate::service::sysex_parser;
use iced::{
    widget::{button, column, container, pick_list, row, scrollable, text, Space},
    Center, Element, Fill, Length, Theme,
};

const VISIBLE_ENTRIES: usize = 200;
#[cfg(any(debug_assertions, feature = "dev-tools"))]
const HEX_PREVIEW_BYTES: usize = 256;

pub fn view_settings(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let title = text("Settings").size(18);

    let appearance_section = build_appearance_section(controller);
    let manual_section = build_manual_section();
    let log_section = build_log_section(controller);

    let about_section = build_about_section();

    let mut content = column![
        title,
        Space::new().height(12),
        about_section,
        Space::new().height(16),
        appearance_section,
        Space::new().height(16),
        manual_section,
        Space::new().height(16),
        log_section,
    ]
    .width(Fill)
    .padding([0, 10]);

    #[cfg(any(debug_assertions, feature = "dev-tools"))]
    if let Some(ref dump) = controller.last_sysex_dump {
        content = content.push(Space::new().height(16));
        content = content.push(build_hex_viewer(dump));
    }

    content = content.push(Space::new().height(12));

    scrollable(content).height(Fill).into()
}

fn build_appearance_section(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let header = text("Appearance & Sync").size(14);

    let theme_row = row![
        text("Theme").size(12),
        Space::new().width(12),
        pick_list(Theme::ALL, Some(&controller.theme), InterfaceMessage::ThemeChanged)
            .text_size(12)
            .padding([4, 8]),
    ]
    .align_y(Center)
    .spacing(8);

    let sync_row = row![
        text("Sync interval").size(12),
        Space::new().width(12),
        pick_list(
            SyncInterval::ALL,
            Some(controller.sync_interval),
            InterfaceMessage::SyncIntervalChanged,
        )
        .text_size(12)
        .padding([4, 8]),
    ]
    .align_y(Center)
    .spacing(8);

    container(
        column![
            header,
            Space::new().height(8),
            theme_row,
            Space::new().height(6),
            sync_row,
        ]
        .padding([12, 14])
        .width(Fill),
    )
    .style(container::rounded_box)
    .width(Fill)
    .into()
}

fn build_manual_section() -> Element<'static, InterfaceMessage> {
    let header = text("Manual").size(14);
    let description = text("Read the user manual for setup and usage instructions.").size(12);

    let open_btn = button(text("Open Manual").size(12))
        .on_press(InterfaceMessage::OpenManual)
        .padding([6, 14])
        .style(button::secondary);

    container(
        column![header, Space::new().height(6), description, Space::new().height(8), open_btn]
            .padding([12, 14])
            .width(Fill),
    )
    .style(container::rounded_box)
    .width(Fill)
    .into()
}

fn build_log_section(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let header = text("Debug Log").size(14);

    let device_label = match &controller.connected_device_name {
        Some(name) => format!("Connected: {}", name),
        None => "Not connected".to_string(),
    };

    let header_row = row![
        header,
        Space::new().width(Fill),
        text(device_label).size(11),
    ]
    .align_y(Center);

    let entries = logger::get_recent_entries(VISIBLE_ENTRIES);
    let count = logger::entry_count();

    let mut log_col = iced::widget::Column::new().spacing(1);

    if count > VISIBLE_ENTRIES {
        log_col = log_col.push(
            text(format!("... ({} earlier entries omitted)", count - VISIBLE_ENTRIES)).size(10),
        );
    }

    for entry in &entries {
        let entry_text = text(format!("{}", entry)).size(11);
        let styled = match entry.level {
            logger::LogLevel::Error => entry_text.color(iced::Color::from_rgb(1.0, 0.3, 0.3)),
            logger::LogLevel::Warn => entry_text.color(iced::Color::from_rgb(1.0, 0.75, 0.2)),
            _ => entry_text,
        };
        log_col = log_col.push(styled);
    }

    if entries.is_empty() {
        log_col = log_col.push(text("No log entries yet.").size(11));
    }

    let log_scroll = scrollable(log_col).height(Length::Fixed(300.0));

    #[allow(unused_mut)]
    let mut actions = row![
        button(text("Copy to Clipboard").size(12))
            .on_press(InterfaceMessage::CopyDebugLog)
            .padding([6, 14])
            .style(button::secondary),
        button(text("Save as File").size(12))
            .on_press(InterfaceMessage::SaveDebugLog)
            .padding([6, 14])
            .style(button::secondary),
    ]
    .spacing(8);

    #[cfg(any(debug_assertions, feature = "dev-tools"))]
    {
        let calibrate_btn = if controller.calibration.is_running() {
            button(text("Calibrating...").size(12))
                .padding([6, 14])
                .style(button::secondary)
        } else {
            button(text("Calibrate SysEx").size(12))
                .on_press(InterfaceMessage::CalibrateStart)
                .padding([6, 14])
                .style(button::primary)
        };
        let digest_btn = button(text("Run Digest").size(12))
            .on_press(InterfaceMessage::DigestRun)
            .padding([6, 14])
            .style(button::secondary);
        actions = actions.push(calibrate_btn);
        actions = actions.push(digest_btn);
    }

    let diagnostics = text(format!(
        "OS: {} {} | Entries: {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        count,
    ))
    .size(10);

    container(
        column![
            header_row,
            Space::new().height(8),
            container(log_scroll)
                .style(container::rounded_box)
                .width(Fill)
                .padding(8),
            Space::new().height(8),
            row![actions, Space::new().width(Fill), diagnostics].align_y(Center),
        ]
        .padding([12, 14])
        .width(Fill),
    )
    .style(container::rounded_box)
    .width(Fill)
    .into()
}

#[cfg(any(debug_assertions, feature = "dev-tools"))]
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

fn build_about_section() -> Element<'static, InterfaceMessage> {
    let header = text("About").size(14);

    let version = text(format!(
        "FLOW 8 MIDI Controller v{}",
        env!("CARGO_PKG_VERSION")
    ))
    .size(12);

    let description = text(
        "A non-official cross-platform desktop MIDI controller for the Behringer FLOW 8 mixer.",
    )
    .size(11);

    let author = text("Author: Abel Rocha Espinosa").size(11);
    let license = text("License: GNU GPLv3").size(11);
    let repo_row = row![
        text("Repository:").size(11),
        Space::new().width(4),
        button(text("github.com/abelroes/flow-8-midi").size(11))
            .on_press(InterfaceMessage::OpenRepository)
            .padding([2, 6])
            .style(button::text),
    ]
    .align_y(Center);

    let donate_btn = button(text("Buy me a beer!").size(13).center())
        .on_press(InterfaceMessage::OpenDonation)
        .padding([10, 24])
        .style(move |theme, status| {
            let mut style = button::primary(theme, status);
            style.background = Some(iced::Background::Color(iced::Color {
                r: 0.96,
                g: 0.65,
                b: 0.14,
                a: 1.0,
            }));
            style.text_color = iced::Color::BLACK;
            style
        });

    let donate_row = column![
        text("If you find this useful, consider supporting the project:").size(11),
        Space::new().height(6),
        donate_btn,
    ];

    container(
        column![
            header,
            Space::new().height(8),
            version,
            Space::new().height(4),
            description,
            Space::new().height(6),
            author,
            license,
            repo_row,
            Space::new().height(8),
            donate_row,
        ]
        .padding([12, 14])
        .width(Fill),
    )
    .style(container::rounded_box)
    .width(Fill)
    .into()
}
