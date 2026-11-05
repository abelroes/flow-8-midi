use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage};
use crate::service::ble_session::SessionState;
use crate::service::midi;
use crate::view::theme::{
    self, ACCENT, ACCENT_BRIGHT, BUTTON, CONNECTED_BG, CONTROL_TRACK, ERROR_BG, PAIRING_BG,
    PRIMARY, SECONDARY, SUCCESS, SURFACE, WARNING_ORANGE,
};
use crate::view::widgets::titlebar;
use iced::{
    widget::{button, column, container, row, text, Space},
    Alignment, Center, Element, Fill, Length,
};

const SUBTITLE_DEFAULT: &str =
    "Choose how to connect. USB MIDI is the stable path. Bluetooth unlocks full mixer sync.";
const SUBTITLE_USB_DETECTED: &str =
    "USB MIDI detected. Choose Bluetooth for full sync, or continue on the stable USB path.";
const BLE_DESC: &str =
    "Full mixer state, meters, names, and snapshots.\nHold PAIR APP on the mixer if it is not advertising.";
const USB_DESC: &str = "Stable control over the USB cable with MIDI CC and SysEx.\n\
• Control levels, EQ, FX, and snapshot load\n\
• No real-time metering or label sync\n\
• Best when the mixer is already plugged in";

pub fn view_connection(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let hero = text("FLOW 8 MIDI Controller").size(28).color(PRIMARY);

    let usb_detected = first_flow8_device();
    let usb_is_present = usb_detected.is_some();
    let subtitle_str = if matches!(controller.session_state, SessionState::Disconnected)
        && usb_is_present
    {
        SUBTITLE_USB_DETECTED
    } else {
        SUBTITLE_DEFAULT
    };
    // Preparing / Pairing / Connected: hero only (no dual-card subtitle).
    let show_subtitle = !matches!(
        controller.session_state,
        SessionState::PairingRequired | SessionState::Preparing | SessionState::Active
    );

    let tick = controller.tick_counter;
    let state_content: Element<'_, InterfaceMessage> = match controller.session_state {
        SessionState::Preparing => view_preparing(),
        SessionState::Scanning => view_scanning(tick),
        SessionState::Connecting
        | SessionState::Subscribing
        | SessionState::WaitingHandshake
        | SessionState::Authenticating
        | SessionState::RequestingState => view_connecting(tick),
        SessionState::PairingRequired => view_pairing(),
        SessionState::Active => view_connected(controller),
        SessionState::Disconnected => {
            if let Some(ref err) = controller.connection_error {
                if err.to_lowercase().contains("cancel") {
                    view_idle(usb_detected)
                } else {
                    view_error(err)
                }
            } else {
                view_idle(usb_detected)
            }
        }
    };

    let mut content = column![hero].align_x(Center).spacing(0);
    if show_subtitle {
        content = content.push(Space::new().height(14));
        content = content.push(text(subtitle_str).size(12).color(SECONDARY));
        content = content.push(Space::new().height(28));
    } else {
        content = content.push(Space::new().height(36));
    }
    content = content.push(state_content);

    let content = content.max_width(1140).padding([40, 20]);

    column![
        titlebar(),
        container(content)
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .style(theme::app_box),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

fn first_flow8_device() -> Option<(String, usize)> {
    midi::list_midi_output_devices()
        .into_iter()
        .find(|d| d.is_flow8)
        .map(|d| (d.name, d.index))
}

fn view_idle(usb_detected: Option<(String, usize)>) -> Element<'static, InterfaceMessage> {
    let usb_title = if usb_detected.is_some() {
        "USB MIDI (detected · legacy)"
    } else {
        "USB MIDI (legacy)"
    };

    let ble_card = connection_card(
        "Bluetooth (full sync)",
        PRIMARY,
        ACCENT,
        BLE_DESC,
        "Connect via Bluetooth",
        theme::BtnKind::Primary,
        InterfaceMessage::ConnectBle,
    );

    let usb_card = connection_card(
        usb_title,
        ACCENT_BRIGHT,
        ACCENT_BRIGHT,
        USB_DESC,
        "Connect via USB MIDI",
        theme::BtnKind::Secondary,
        InterfaceMessage::ConnectUsb,
    );

    let cards = row![ble_card, Space::new().width(20), usb_card]
        .align_y(Alignment::Start);

    let mut col = column![cards].align_x(Center).spacing(0);

    if let Some((name, index)) = usb_detected {
        col = col.push(Space::new().height(16));
        col = col.push(
            text(format!("Detected: {name} (port {index})"))
                .size(11)
                .color(BUTTON),
        );
    }

    #[cfg(target_os = "windows")]
    {
        col = col.push(Space::new().height(24));
        col = col.push(windows_bt_warning());
    }

    col.into()
}

fn connection_card<'a>(
    title: &'a str,
    title_color: iced::Color,
    border_color: iced::Color,
    desc: &'a str,
    cta: &'a str,
    cta_kind: theme::BtnKind,
    on_press: InterfaceMessage,
) -> Element<'a, InterfaceMessage> {
    let cta_btn = button(text(cta).size(12).center())
        .on_press(on_press)
        .width(Length::Fixed(200.0))
        .height(Length::Fixed(40.0))
        .style(theme::btn(cta_kind));

    container(
        column![
            text(title).size(14).color(title_color),
            Space::new().height(14),
            text(desc).size(12).color(SECONDARY),
            Space::new().height(Fill),
            container(cta_btn).center_x(Fill),
        ]
        .padding([20, 30])
        .height(Fill),
    )
    .width(Length::Fixed(560.0))
    .height(Length::Fixed(200.0))
    .style(move |_t| container::Style {
        background: Some(iced::Background::Color(SURFACE)),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

#[cfg(target_os = "windows")]
fn windows_bt_warning() -> Element<'static, InterfaceMessage> {
    container(
        column![
            row![
                text("⚠").size(20).color(WARNING_ORANGE),
                Space::new().width(14),
                text("Windows Bluetooth tradeoff")
                    .size(14)
                    .color(WARNING_ORANGE),
            ]
            .align_y(Center),
            Space::new().height(8),
            text(
                "While Bluetooth is connected, other devices on the same adapter pause. \
                 Prefer a wired keyboard/mouse. The first connection may ask for Administrator \
                 permission. Disconnecting restores Windows Bluetooth.",
            )
            .size(12)
            .color(PRIMARY),
        ]
        .padding([18, 20]),
    )
    .width(Length::Fixed(1120.0))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(PAIRING_BG)),
        border: iced::Border {
            color: WARNING_ORANGE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn view_preparing() -> Element<'static, InterfaceMessage> {
    let card = container(
        column![
            row![
                text("⚙").size(28).color(WARNING_ORANGE),
                Space::new().width(15),
                text("Preparing Bluetooth")
                    .size(18)
                    .color(WARNING_ORANGE),
            ]
            .align_y(Center),
            Space::new().height(14),
            text(
                "Approve the User Account Control prompt, then allow the one-time driver \
                 certificate trust. Waiting…",
            )
            .size(13)
            .color(PRIMARY),
            Space::new().height(16),
            text("Other Bluetooth devices on this PC will pause for the session.")
                .size(11)
                .color(BUTTON),
        ]
        .padding([20, 30]),
    )
    .width(Length::Fixed(720.0))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(PAIRING_BG)),
        border: iced::Border {
            color: WARNING_ORANGE,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    column![
        card,
        Space::new().height(40),
        button(text("Cancel").size(12).center())
            .on_press(InterfaceMessage::CancelConnection)
            .width(Length::Fixed(160.0))
            .height(Length::Fixed(36.0))
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_x(Center)
    .into()
}

fn scanning_dots(tick: u32) -> &'static str {
    match (tick / 2) % 3 {
        0 => "● ○ ○",
        1 => "○ ● ○",
        _ => "○ ○ ●",
    }
}

fn view_scanning(tick: u32) -> Element<'static, InterfaceMessage> {
    #[cfg(target_os = "windows")]
    let status = "Searching… PC Bluetooth in exclusive use until you disconnect.";
    #[cfg(not(target_os = "windows"))]
    let status = "Searching for Bluetooth devices…";

    column![
        button(text("Scanning for FLOW 8…").size(14).center())
            .width(Length::Fixed(280.0))
            .height(Length::Fixed(44.0))
            .style(theme::btn(theme::BtnKind::Primary)),
        Space::new().height(16),
        text(scanning_dots(tick)).size(14).color(SECONDARY),
        Space::new().height(10),
        text(status).size(11).color(SECONDARY),
        Space::new().height(36),
        button(text("Retry USB MIDI").size(12).center())
            .on_press(InterfaceMessage::ConnectUsb)
            .width(Length::Fixed(200.0))
            .height(Length::Fixed(40.0))
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_x(Center)
    .into()
}

fn view_connecting(tick: u32) -> Element<'static, InterfaceMessage> {
    column![
        button(text("Connecting…").size(14).center())
            .width(Length::Fixed(280.0))
            .height(Length::Fixed(44.0))
            .style(theme::btn(theme::BtnKind::Secondary)),
        Space::new().height(16),
        text(scanning_dots(tick)).size(14).color(SECONDARY),
        Space::new().height(10),
        text("Establishing BLE session…")
            .size(11)
            .color(SECONDARY),
        Space::new().height(36),
        button(text("Cancel").size(12).center())
            .on_press(InterfaceMessage::CancelConnection)
            .width(Length::Fixed(160.0))
            .height(Length::Fixed(36.0))
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_x(Center)
    .into()
}

fn view_error(err: &str) -> Element<'_, InterfaceMessage> {
    let short = if err.len() > 160 {
        format!("{}…", &err[..160])
    } else {
        err.to_string()
    };

    let card = container(
        column![
            row![
                text("✗").size(28).color(theme::ERROR),
                Space::new().width(16),
                text("Connection Failed").size(18).color(theme::ERROR),
            ]
            .align_y(Center),
            Space::new().height(12),
            text(short).size(12).color(PRIMARY),
        ]
        .padding([22, 30]),
    )
    .width(Length::Fixed(600.0))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(ERROR_BG)),
        border: iced::Border {
            color: theme::ERROR,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    column![
        card,
        Space::new().height(28),
        row![
            button(text("Retry Bluetooth").size(12).center())
                .on_press(InterfaceMessage::ConnectBle)
                .width(Length::Fixed(200.0))
                .height(Length::Fixed(40.0))
                .style(theme::btn(theme::BtnKind::Primary)),
            Space::new().width(20),
            button(text("Try USB MIDI").size(12).center())
                .on_press(InterfaceMessage::ConnectUsb)
                .width(Length::Fixed(160.0))
                .height(Length::Fixed(40.0))
                .style(theme::btn(theme::BtnKind::Secondary)),
        ]
        .align_y(Center),
    ]
    .align_x(Center)
    .into()
}

fn view_pairing() -> Element<'static, InterfaceMessage> {
    let card = container(
        column![
            row![
                text("⚠").size(28).color(WARNING_ORANGE),
                Space::new().width(15),
                text("Pairing Required").size(18).color(WARNING_ORANGE),
            ]
            .align_y(Center),
            Space::new().height(14),
            text(
                "The mixer does not recognize this device. Press and hold the PAIR APP button \
                 on your FLOW 8 mixer, then the connection will complete automatically.",
            )
            .size(13)
            .color(PRIMARY),
            Space::new().height(16),
            text("Waiting… hold PAIR APP on the mixer")
                .size(11)
                .color(BUTTON),
            Space::new().height(10),
            progress_bar(CONTROL_TRACK, WARNING_ORANGE, 0.66),
        ]
        .padding([20, 30]),
    )
    .width(Length::Fixed(720.0))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(PAIRING_BG)),
        border: iced::Border {
            color: WARNING_ORANGE,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    column![
        card,
        Space::new().height(40),
        button(text("Cancel").size(12).center())
            .on_press(InterfaceMessage::CancelConnection)
            .width(Length::Fixed(160.0))
            .height(Length::Fixed(36.0))
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_x(Center)
    .into()
}

fn view_connected(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let fw = controller
        .firmware_version
        .as_deref()
        .unwrap_or("—");
    let proto = controller
        .protocol_version
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".into());
    let info = format!("Bluetooth (BLE) • Firmware {fw} • Protocol v{proto}");

    let card = container(
        column![
            row![
                text("✓").size(36).color(SUCCESS),
                Space::new().width(22),
                column![
                    text("Connected to FLOW 8").size(18).color(SUCCESS),
                    Space::new().height(10),
                    text(info).size(12).color(SECONDARY),
                    Space::new().height(8),
                    text("Receiving mixer state…").size(12).color(SECONDARY),
                    Space::new().height(12),
                    progress_bar(CONTROL_TRACK, SUCCESS, 0.75),
                ],
            ]
            .align_y(Alignment::Start),
            Space::new().height(16),
            text("Opening mixer in a moment…")
                .size(11)
                .color(BUTTON),
        ]
        .padding([24, 30]),
    )
    .width(Length::Fixed(560.0))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(CONNECTED_BG)),
        border: iced::Border {
            color: SUCCESS,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    });

    column![card].align_x(Center).into()
}

fn progress_bar(
    track: iced::Color,
    fill: iced::Color,
    ratio: f32,
) -> Element<'static, InterfaceMessage> {
    let ratio = ratio.clamp(0.05, 1.0);
    let filled = (ratio * 100.0).round() as u16;
    let empty = 100u16.saturating_sub(filled).max(1);
    container(
        row![
            container(Space::new())
                .width(Length::FillPortion(filled))
                .height(Length::Fixed(4.0))
                .style(move |_t| container::Style {
                    background: Some(iced::Background::Color(fill)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            container(Space::new())
                .width(Length::FillPortion(empty))
                .height(Length::Fixed(4.0)),
        ],
    )
    .width(Fill)
    .height(Length::Fixed(4.0))
    .style(move |_t| container::Style {
        background: Some(iced::Background::Color(track)),
        border: iced::Border {
            radius: 2.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
