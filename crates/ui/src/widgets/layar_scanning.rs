use crate::icons::{icon, icon_colored, ICON_SPINNER, ICON_TIMES};
use crate::theme::{
    card_style, chip_style, danger_button, dynamic_progress_bar_style, COLOR_CYAN, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::widgets::size_bar::{format_bytes, format_count};
use iced::widget::{button, column, container, progress_bar, row, text, Space};
use iced::{Element, Length};

pub fn view_scanning<'a, Message: 'a + Clone + 'static>(
    drive_label: &str,
    items: u64,
    bytes: u64,
    elapsed_ms: u64,
    on_cancel: Message,
) -> Element<'a, Message> {
    let speed = if elapsed_ms > 0 {
        (items as f64 / (elapsed_ms as f64 / 1000.0)) as u64
    } else {
        0
    };

    let duration_sec = elapsed_ms as f64 / 1000.0;

    // Header dengan spinner
    let title_row = row![
        icon_colored(ICON_SPINNER, COLOR_CYAN).size(26),
        Space::with_width(Length::Fixed(12.0)),
        text(format!("Memindai {}", drive_label))
            .size(22)
            .color(TEXT_PRIMARY),
    ]
    .align_y(iced::Alignment::Center);

    let bar = progress_bar(0.0..=1.0, 0.5)
        .height(8)
        .width(Length::Fixed(460.0))
        .style(dynamic_progress_bar_style(0.5));

    // Kartu Metrik Utama yang Besar dan Elegan
    let items_card = container(
        column![
            text(format_count(items as u32))
                .size(26)
                .color(TEXT_PRIMARY),
            text("Total Berkas & Folder")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2)
        .align_x(iced::Alignment::Center),
    )
    .padding([14, 28])
    .style(chip_style);

    let bytes_card = container(
        column![
            text(format_bytes(bytes))
                .size(26)
                .color(COLOR_CYAN),
            text("Ruang Terpakai")
                .size(11)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2)
        .align_x(iced::Alignment::Center),
    )
    .padding([14, 28])
    .style(chip_style);

    let primary_stats = row![
        items_card,
        Space::with_width(Length::Fixed(16.0)),
        bytes_card,
    ]
    .align_y(iced::Alignment::Center);

    let secondary_info = text(format!(
        "Kecepatan: {} berkas/dtk  •  Durasi: {:.1}s",
        format_count(speed as u32),
        duration_sec
    ))
    .size(12)
    .color(TEXT_MUTED);

    let cancel_btn = button(
        row![
            icon(ICON_TIMES).size(12),
            Space::with_width(Length::Fixed(8.0)),
            text("Batalkan").size(13),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([8, 22])
    .style(danger_button)
    .on_press(on_cancel);

    let card = container(
        column![
            title_row,
            Space::with_height(Length::Fixed(4.0)),
            bar,
            Space::with_height(Length::Fixed(12.0)),
            primary_stats,
            Space::with_height(Length::Fixed(6.0)),
            secondary_info,
            Space::with_height(Length::Fixed(14.0)),
            cancel_btn,
        ]
        .spacing(8)
        .padding(32)
        .align_x(iced::Alignment::Center),
    )
    .style(card_style);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
