use crate::service::ble::BleStatus;
use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage, page::Page};
use iced::{
    widget::{button, container, row, text, tooltip, tooltip::Position, Space},
    Center, Element, Fill,
};

const NAV_PAGES: [Page; 5] = [Page::Mixer, Page::Eq, Page::Sends, Page::Fx, Page::Snapshots];

const SYNCED_COLOR: iced::Color = iced::Color {
    r: 0.2,
    g: 0.8,
    b: 0.4,
    a: 1.0,
};

use super::widgets::UNSYNCED_COLOR;

pub fn view_nav_bar(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let mut tabs = row![].spacing(4).align_y(Center);

    for page in NAV_PAGES {
        let label = text(format!("{}", page)).size(13);
        let btn = if page == controller.current_page {
            button(label).padding([6, 16])
        } else {
            button(label)
                .on_press(InterfaceMessage::NavigateTo(page))
                .padding([6, 16])
                .style(button::secondary)
        };
        tabs = tabs.push(btn);
    }

    tabs = tabs.push(Space::new().width(Fill));

    let last_sync_label = format_last_sync(controller.last_sync_time);
    tabs = tabs.push(
        container(text(last_sync_label).size(9))
            .padding([4, 6]),
    );
    tabs = tabs.push(Space::new().width(4));

    let is_synced = controller.is_globally_synced();
    let (sync_color, sync_text) = if is_synced {
        (SYNCED_COLOR, "Synced")
    } else {
        (UNSYNCED_COLOR, "Unsynced")
    };

    let sync_badge = container(
        row![
            text("\u{25CF}").size(10).color(sync_color),
            text(sync_text).size(10),
        ]
        .spacing(4)
        .align_y(Center),
    )
    .padding([4, 10])
    .style(container::rounded_box);

    let sync_tip = if is_synced {
        "All parameters are in sync with the mixer"
    } else {
        "Yellow dot means parameters are unsynced with the mixer"
    };

    tabs = tabs.push(
        tooltip(
            sync_badge,
            container(text(sync_tip).size(11))
                .padding(6)
                .style(container::rounded_box),
            Position::Bottom,
        )
        .gap(4),
    );

    tabs = tabs.push(Space::new().width(4));

    if controller.ble_status == BleStatus::Connected {
        let sync_style = if controller
            .sync_last_click
            .map(|t| t.elapsed().as_millis() < 1000)
            .unwrap_or(false)
        {
            button::primary
        } else {
            button::secondary
        };

        tabs = tabs.push(
            tooltip(
                button(text("Sync").size(11))
                    .on_press(InterfaceMessage::BleRequestDump)
                    .padding([4, 10])
                    .style(sync_style),
                container(text("Click to request state dump from mixer").size(11))
                    .padding(6)
                    .style(container::rounded_box),
                Position::Bottom,
            )
            .gap(4),
        );
        tabs = tabs.push(Space::new().width(4));
    }

    let bt_led = build_ble_indicator(controller);
    tabs = tabs.push(bt_led);
    tabs = tabs.push(Space::new().width(4));

    let ble_active = matches!(
        controller.ble_status,
        BleStatus::Connected | BleStatus::Scanning | BleStatus::Connecting | BleStatus::Authenticating
    );

    let disconnect_btn = {
        let base = button(text("Disconnect").size(11))
            .padding([4, 10])
            .style(button::danger);
        if ble_active {
            base.on_press(InterfaceMessage::Disconnect)
        } else {
            base
        }
    };

    tabs = tabs.push(
        tooltip(
            disconnect_btn,
            container(text("Disconnect BLE from mixer").size(11))
                .padding(6)
                .style(container::rounded_box),
            Position::Bottom,
        )
        .gap(4),
    );

    tabs = tabs.push(Space::new().width(8));

    let settings_btn = if controller.current_page == Page::Settings {
        button(text("\u{2699}").size(13)).padding([4, 8])
    } else {
        button(text("\u{2699}").size(13))
            .on_press(InterfaceMessage::NavigateTo(Page::Settings))
            .padding([4, 8])
            .style(button::secondary)
    };
    tabs = tabs.push(
        tooltip(
            settings_btn,
            container(text("Settings").size(11))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Bottom,
        )
        .gap(4),
    );

    container(tabs)
        .width(Fill)
        .padding([8, 12])
        .into()
}

const BLE_CONNECTED_COLOR: iced::Color = iced::Color {
    r: 0.3,
    g: 0.6,
    b: 1.0,
    a: 1.0,
};

const BLE_DISCONNECTED_COLOR: iced::Color = iced::Color {
    r: 0.4,
    g: 0.4,
    b: 0.4,
    a: 1.0,
};

const BLE_BUSY_COLOR: iced::Color = iced::Color {
    r: 1.0,
    g: 0.8,
    b: 0.2,
    a: 1.0,
};

const BLE_ERROR_COLOR: iced::Color = iced::Color {
    r: 1.0,
    g: 0.3,
    b: 0.3,
    a: 1.0,
};

const BLE_BLINK_DIM: iced::Color = iced::Color {
    r: 0.5,
    g: 0.4,
    b: 0.1,
    a: 0.4,
};

fn build_ble_indicator(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let is_busy = matches!(
        controller.ble_status,
        BleStatus::Scanning | BleStatus::Connecting | BleStatus::Authenticating
    );

    let color = match controller.ble_status {
        BleStatus::Connected => BLE_CONNECTED_COLOR,
        _ if is_busy => {
            if controller.tick_counter % 5 < 3 {
                BLE_BUSY_COLOR
            } else {
                BLE_BLINK_DIM
            }
        }
        BleStatus::Error => BLE_ERROR_COLOR,
        _ => BLE_DISCONNECTED_COLOR,
    };

    let tip = match controller.ble_status {
        BleStatus::Connected => "BLE connected",
        BleStatus::Scanning => "Scanning for FLOW 8 LE...",
        BleStatus::Connecting => "Connecting to FLOW 8 LE...",
        BleStatus::Authenticating => "Authenticating...",
        BleStatus::Error => "BLE error — double-click to retry",
        BleStatus::Disconnected => "BLE disconnected — double-click to reconnect",
        BleStatus::Unavailable => "No Bluetooth adapter",
    };

    let content = row![
        text("\u{25CF}").size(10).color(color),
        text("BT").size(10),
    ]
    .spacing(3)
    .align_y(Center);

    tooltip(
        button(content)
            .on_press(InterfaceMessage::BleConnect)
            .padding([4, 8])
            .style(button::text),
        container(text(tip).size(11))
            .padding(6)
            .style(container::rounded_box),
        Position::Bottom,
    )
    .gap(4)
    .into()
}

fn format_last_sync(last_sync: Option<std::time::Instant>) -> String {
    match last_sync {
        None => "Never synced".to_string(),
        Some(t) => {
            let secs = t.elapsed().as_secs();
            if secs < 60 {
                "Last sync: less than 1m ago".to_string()
            } else {
                format!("Last sync: {}m ago", secs / 60)
            }
        }
    }
}
