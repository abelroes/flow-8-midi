use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage};
use iced::{
    widget::{button, column, container, row, text, Space},
    Center, Element, Fill, Length,
};

pub fn view_device_select(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let title = text("FLOW 8 MIDI Controller").size(28);

    let error_msg = controller
        .connection_error
        .as_deref()
        .unwrap_or("Searching for FLOW 8...");

    let error_text = text(error_msg).size(14);

    let retry_btn = button(text("Retry").size(14))
        .on_press(InterfaceMessage::RetryConnection)
        .padding([10, 32]);

    let log_actions = row![
        button(text("Copy Log").size(12))
            .on_press(InterfaceMessage::CopyDebugLog)
            .padding([6, 14])
            .style(button::secondary),
        button(text("Save Log").size(12))
            .on_press(InterfaceMessage::SaveDebugLog)
            .padding([6, 14])
            .style(button::secondary),
    ]
    .spacing(8);

    let content = column![
        title,
        Space::new().height(24),
        error_text,
        Space::new().height(16),
        retry_btn,
        Space::new().height(24),
        log_actions,
    ]
    .align_x(Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Fill)
        .into()
}
