use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage, page::Page};
use crate::service::ble_session::SessionState;
use crate::view::theme::{self, ACCENT_BRIGHT, NAV_HEIGHT, PRIMARY, TERTIARY};
use crate::view::widgets::{nav_tab, view_tab};
use iced::{
    widget::{button, container, row, text, Space},
    Center, Element, Fill, Length,
};

pub fn view_nav_bar(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let mut nav = row![].spacing(8).padding([0, 16]).align_y(Center);

    let tabs: &[(Page, &str)] = &[
        (Page::Home, "Home"),
        (Page::Mixer, "Main"),
        (Page::Setup, "Setup"),
    ];

    for &(page, label) in tabs {
        let is_active = controller.current_page == page
            || (page == Page::Mixer && controller.current_page.is_main_section());
        nav = nav.push(nav_tab(
            label,
            is_active,
            InterfaceMessage::NavigateTo(page),
        ));
    }

    if controller.current_page.is_main_section() {
        nav = nav.push(Space::new().width(16));
        for &(page, label) in &[(Page::Mixer, "Mixer"), (Page::Stage, "Stage")] {
            nav = nav.push(view_tab(
                label,
                controller.current_page == page,
                InterfaceMessage::NavigateTo(page),
            ));
        }
    }

    nav = nav.push(Space::new().width(Fill));

    let (dot_color, bt_text) = match controller.session_state {
        SessionState::Active => (ACCENT_BRIGHT, "BT"),
        SessionState::Disconnected => (TERTIARY, "BT"),
        _ => (ACCENT_BRIGHT, "BT"),
    };

    let bt_dot = container(Space::new())
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0))
        .style(move |_t| container::Style {
            background: Some(iced::Background::Color(dot_color)),
            border: iced::Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    nav = nav.push(
        row![bt_dot, Space::new().width(6), text(bt_text).size(12).color(PRIMARY)]
            .align_y(Center),
    );
    nav = nav.push(Space::new().width(16));

    let disconnect = button(text("Disconnect").size(12).center())
        .width(Length::Fixed(100.0))
        .height(Length::Fixed(32.0))
        .on_press(InterfaceMessage::Disconnect)
        .style(theme::btn(theme::BtnKind::Danger));
    nav = nav.push(disconnect);

    container(nav)
        .width(Fill)
        .height(Length::Fixed(NAV_HEIGHT))
        .align_y(Center)
        .style(theme::navbar_box)
        .into()
}
