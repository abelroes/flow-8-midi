use crate::model::{flow8::{FLOW8Controller, SNAPSHOT_COUNT}, message::InterfaceMessage};
use iced::{
    widget::{button, column, container, row, text, Space},
    Center, Element, Fill,
};

const GRID_COLS: usize = 4;

const EMPTY_NUM_COLOR: iced::Color = iced::Color {
    r: 0.45,
    g: 0.45,
    b: 0.45,
    a: 1.0,
};

const AVAILABLE_NUM_COLOR: iced::Color = iced::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

const AVAILABLE_NAME_COLOR: iced::Color = iced::Color {
    r: 0.85,
    g: 0.85,
    b: 0.85,
    a: 1.0,
};

pub fn view_snapshots(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let mut rows: Vec<Element<'_, InterfaceMessage>> = Vec::new();

    for chunk_start in (0..SNAPSHOT_COUNT).step_by(GRID_COLS) {
        let mut row_items: Vec<Element<'_, InterfaceMessage>> = Vec::new();
        for i in chunk_start..((chunk_start + GRID_COLS).min(SNAPSHOT_COUNT)) {
            let snapshot_num = i + 1;
            let name = controller
                .snapshot_names
                .get(i)
                .and_then(|n| n.as_ref());

            let has_name = name.is_some();

            let btn_content = match name {
                Some(n) => column![
                    text(format!("{:02}", snapshot_num))
                        .size(20)
                        .align_x(Center)
                        .color(AVAILABLE_NUM_COLOR),
                    text(n.as_str())
                        .size(15)
                        .align_x(Center)
                        .color(AVAILABLE_NAME_COLOR),
                ]
                .align_x(Center)
                .spacing(4),
                None => column![
                    text(format!("{:02}", snapshot_num))
                        .size(20)
                        .align_x(Center)
                        .color(EMPTY_NUM_COLOR),
                    text(" ").size(15),
                ]
                .align_x(Center)
                .spacing(4),
            };

            let btn = button(btn_content)
                .on_press(InterfaceMessage::LoadSnapshot(i as u8))
                .padding([16, 16])
                .width(Fill)
                .style(if has_name { button::primary } else { button::secondary });

            row_items.push(btn.into());
        }

        let remaining = GRID_COLS - row_items.len();
        for _ in 0..remaining {
            row_items.push(Space::new().width(Fill).into());
        }

        rows.push(
            row(row_items)
                .spacing(8)
                .width(Fill)
                .into(),
        );
    }

    let grid = column(rows).spacing(8);

    let reset_btn = button(
        text("Reset to Default").size(14).align_x(Center),
    )
    .on_press(InterfaceMessage::ResetMixer)
    .padding([12, 24])
    .width(Fill)
    .style(button::danger);

    let content = column![
        text("Mixer Snapshots").size(18),
        Space::new().height(12),
        grid,
        Space::new().height(16),
        reset_btn,
    ]
    .align_x(Center)
    .spacing(4)
    .padding(20)
    .width(Fill);

    container(content)
        .width(Fill)
        .height(Fill)
        .align_x(Center)
        .into()
}
