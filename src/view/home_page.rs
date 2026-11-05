use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage, page::Page};
use crate::view::theme::{self, PRIMARY, SECONDARY, WARNING, WARNING_BG};
use iced::{
    widget::{button, column, container, row, text, Space},
    Center, Element, Fill, Length,
};

pub fn view_home(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let mut content = column![].align_x(Center).max_width(600).width(Fill);

    if let Some(ref fw) = controller.firmware_version {
        if !controller.firmware_notice_dismissed {
            let notice_text = format!(
                "Firmware update available: {}. Visit behringer.com for details.",
                fw
            );
            let notice = container(
                row![
                    text("⚠").size(14).color(WARNING),
                    Space::new().width(8),
                    text(notice_text).size(11).color(WARNING),
                    Space::new().width(Fill),
                    button(text("×").size(16).color(SECONDARY))
                        .on_press(InterfaceMessage::DismissFirmwareNotice)
                        .style(theme::text_link),
                ]
                .align_y(Center)
                .padding([10, 18]),
            )
            .width(Fill)
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(WARNING_BG)),
                border: iced::Border {
                    color: WARNING,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

            content = content.push(notice);
            content = content.push(Space::new().height(24));
        }
    }

    content = content.push(text("Welcome — FLOW 8").size(26).color(PRIMARY));
    content = content.push(Space::new().height(48));

    for (label, msg, primary) in [
        (
            "Continue Session",
            InterfaceMessage::NavigateTo(Page::Mixer),
            true,
        ),
        ("Start New", InterfaceMessage::ResetMixer, false),
        (
            "Load Snapshot",
            InterfaceMessage::NavigateTo(Page::Snapshots),
            false,
        ),
        (
            "Assisted Setup",
            InterfaceMessage::NavigateTo(Page::Setup),
            false,
        ),
    ] {
        let btn = button(text(label).size(12).center())
            .width(Length::Fixed(360.0))
            .height(Length::Fixed(40.0))
            .on_press(msg)
            .style(if primary {
                theme::btn(theme::BtnKind::Primary)
            } else {
                theme::btn(theme::BtnKind::Secondary)
            });
        content = content.push(btn);
        content = content.push(Space::new().height(10));
    }

    content = content.push(Space::new().height(16));
    content = content.push(
        row![
            button(text("Open Main").size(13))
                .on_press(InterfaceMessage::NavigateTo(Page::Mixer))
                .style(theme::text_link),
            Space::new().width(32),
            button(text("Setup").size(13))
                .on_press(InterfaceMessage::NavigateTo(Page::Setup))
                .style(theme::text_link),
        ]
        .align_y(Center),
    );

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Fill)
        .padding([20, 24])
        .style(theme::app_box)
        .into()
}
