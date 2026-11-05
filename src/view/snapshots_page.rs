use crate::model::{
    flow8::{FLOW8Controller, SNAPSHOT_COUNT},
    message::InterfaceMessage,
};
use crate::view::setup_nav::{card_section, setup_header};
use crate::view::theme::{self, PRIMARY, SECONDARY, TERTIARY, WARNING, WARNING_BG};
use iced::{
    widget::{button, column, container, row, text, Space},
    Center, Element, Fill, Length,
};

const GRID_COLS: usize = 4;
const SCOPE_LABELS: [&str; 4] = ["Channels", "FX", "Main / Mon", "Routing"];

pub fn view_snapshots(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let header = setup_header(2);

    let notice = container(
        column![
            text("Snapshot names unavailable").size(12).color(WARNING),
            text(
                "Windows BLE subscribe may fail — slots still load by number. Names show as empty until reconnect succeeds.",
            )
            .size(11)
            .color(WARNING),
        ]
        .spacing(4)
        .padding([10, 14]),
    )
    .width(Fill)
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(WARNING_BG)),
        border: iced::Border {
            color: WARNING,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    });

    let mut rows: Vec<Element<'_, InterfaceMessage>> = Vec::new();

    for chunk_start in (0..SNAPSHOT_COUNT).step_by(GRID_COLS) {
        let mut row_items: Vec<Element<'_, InterfaceMessage>> = Vec::new();
        for i in chunk_start..((chunk_start + GRID_COLS).min(SNAPSHOT_COUNT)) {
            let snapshot_num = i + 1;
            let name = controller.snapshot_names.get(i).and_then(|n| n.as_ref());
            let has_name = name.is_some();
            let slot = i as u8;

            let title = text(format!("{:02}", snapshot_num))
                .size(18)
                .color(PRIMARY);
            let subtitle = match name {
                Some(n) => text(n.as_str()).size(13).color(PRIMARY),
                None => text("empty").size(13).color(TERTIARY),
            };

            let actions = row![
                button(text("Save").size(10).center())
                    .padding([4, 8])
                    .style(theme::btn(theme::BtnKind::Quiet)),
                Space::new().width(4),
                button(text("Del").size(10).center())
                    .padding([4, 8])
                    .style(theme::btn(theme::BtnKind::Danger)),
            ];

            let body = column![
                row![actions, Space::new().width(Fill)].align_y(Center),
                Space::new().height(8),
                title,
                Space::new().height(4),
                subtitle,
                Space::new().height(Fill),
                text(if has_name { "Load" } else { "Rename…" })
                    .size(11)
                    .color(SECONDARY),
            ]
            .padding(12)
            .height(Length::Fixed(120.0));

            let btn = button(body)
                .on_press(InterfaceMessage::LoadSnapshot(slot))
                .width(Fill)
                .style(if has_name {
                    theme::btn(theme::BtnKind::Primary)
                } else {
                    theme::btn(theme::BtnKind::Secondary)
                });

            row_items.push(btn.into());
        }

        rows.push(row(row_items).spacing(10).width(Fill).into());
    }

    let mut scope_col = column![text("Snapshot scope").size(14).color(PRIMARY)];
    for (i, label) in SCOPE_LABELS.iter().enumerate() {
        let on = controller.snapshot_scope[i];
        scope_col = scope_col.push(Space::new().height(8));
        scope_col = scope_col.push(
            row![
                text(if on { "✓" } else { "○" })
                    .size(13)
                    .color(if on { theme::SUCCESS } else { TERTIARY }),
                Space::new().width(8),
                button(text(*label).size(12).color(PRIMARY))
                    .on_press(InterfaceMessage::ToggleSnapshotScope(i as u8))
                    .style(theme::text_link),
            ]
            .align_y(Center),
        );
    }
    let scope_card = container(scope_col.padding(16))
        .width(Fill)
        .style(theme::surface_box);

    let library = card_section(
        "Snapshot Library (local)",
        column![
            text("No local library snapshots yet.")
                .size(12)
                .color(SECONDARY),
            Space::new().height(10),
            row![
                button(text("Add from path").size(12).center())
                    .padding([8, 12])
                    .style(theme::btn(theme::BtnKind::Secondary)),
                Space::new().width(8),
                button(text("Import path").size(12).center())
                    .padding([8, 12])
                    .style(theme::btn(theme::BtnKind::Secondary)),
            ],
            Space::new().height(10),
            text("Sort by name     Sort by date")
                .size(11)
                .color(TERTIARY),
            Space::new().height(8),
            text("Library files live under ~/.local/share/flow-8-midi/snapshots/")
                .size(11)
                .color(TERTIARY),
        ]
        .into(),
    );

    let reset = button(text("Reset to Default").size(12).center())
        .width(Fill)
        .height(Length::Fixed(40.0))
        .on_press(InterfaceMessage::ResetMixer)
        .style(theme::btn(theme::BtnKind::Danger));

    let content = column![
        header,
        column![
            notice,
            Space::new().height(12),
            text("Mixer Snapshots").size(14).color(PRIMARY),
            Space::new().height(10),
            column(rows).spacing(10),
            Space::new().height(16),
            row![scope_card, Space::new().width(12), library].width(Fill),
            Space::new().height(16),
            reset,
        ]
        .padding([0, 10]),
    ]
    .width(Fill);

    container(content)
        .width(Fill)
        .height(Fill)
        .style(theme::app_box)
        .into()
}
