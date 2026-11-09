use crate::icons::{
    icon, icon_colored, ICON_ARROW_UP, ICON_CHEVRON_RIGHT, ICON_FOLDER, ICON_HDD,
};
use crate::theme::{
    ghost_button, primary_button, secondary_button, COLOR_CYAN, COLOR_FOLDER, TEXT_MUTED,
    TEXT_SECONDARY,
};
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas};
use iced::widget::{button, row, scrollable, text, Space};
use iced::{Element, Length};

/// Merender bar navigasi hierarki (Breadcrumb) modern berbentuk path chips.
pub fn breadcrumb_bar<'a, Message: 'a + Clone + 'static>(
    model: &PohonSistemBerkas,
    current_dir: Option<IdEntri>,
    on_navigate: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_navigate_up: Message,
) -> Element<'a, Message> {
    let mut bar = row![].spacing(6).align_y(iced::Alignment::Center);

    // Tombol "Naik Satu Level"
    let can_go_up = current_dir.map_or(false, |id| id.0 != 0);
    let mut up_btn = button(
        row![
            icon(ICON_ARROW_UP).size(12),
            Space::with_width(Length::Fixed(6.0)),
            text("Ke Atas").size(12),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([5, 10])
    .style(secondary_button);

    if can_go_up {
        up_btn = up_btn.on_press(on_navigate_up);
    }

    bar = bar.push(up_btn);
    bar = bar.push(Space::with_width(Length::Fixed(4.0)));

    if let Some(curr) = current_dir {
        let chain = model.ancestor_chain(curr);
        let len = chain.len();

        for (idx, &node_id) in chain.iter().enumerate() {
            if idx > 0 {
                bar = bar.push(
                    icon_colored(ICON_CHEVRON_RIGHT, TEXT_MUTED).size(10),
                );
            }

            let is_root = node_id.0 == 0;
            let name = if is_root {
                let raw = model.nama(node_id);
                if raw.is_empty() || raw == "/" {
                    "Root".to_string()
                } else {
                    raw.to_string()
                }
            } else {
                model.nama(node_id).to_string()
            };

            let is_last = idx + 1 == len;

            let chip_content = if is_root {
                row![
                    icon_colored(ICON_HDD, COLOR_CYAN).size(12),
                    Space::with_width(Length::Fixed(6.0)),
                    text(name).size(12),
                ]
                .align_y(iced::Alignment::Center)
            } else {
                row![
                    icon_colored(ICON_FOLDER, COLOR_FOLDER).size(12),
                    Space::with_width(Length::Fixed(6.0)),
                    text(name).size(12),
                ]
                .align_y(iced::Alignment::Center)
            };

            let chip_btn = button(chip_content)
                .padding([4, 10])
                .style(if is_last {
                    primary_button
                } else {
                    ghost_button
                })
                .on_press(on_navigate(node_id));

            bar = bar.push(chip_btn);
        }
    } else {
        bar = bar.push(text("Belum ada direktori yang dipilih").size(13).color(TEXT_SECONDARY));
    }

    scrollable(bar)
        .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
        .width(Length::Fill)
        .into()
}
