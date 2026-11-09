use crate::icons::{icon_colored, ICON_ROCKET, ICON_SPINNER};
use crate::theme::{card_style, COLOR_CYAN, COLOR_PRIMARY, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY};
use iced::widget::{column, container, row, text, Space};
use iced::{Element, Length};

pub fn view_loading<'a, Message: 'a>() -> Element<'a, Message> {
    let logo_row = row![
        icon_colored(ICON_ROCKET, COLOR_PRIMARY).size(36),
        Space::with_width(Length::Fixed(12.0)),
        column![
            text("Fast Disk Analyze")
                .size(26)
                .color(TEXT_PRIMARY),
            text("High-Performance Disk Scanner")
                .size(13)
                .color(TEXT_SECONDARY),
        ]
        .spacing(2),
    ]
    .align_y(iced::Alignment::Center);

    let spinner_row = row![
        icon_colored(ICON_SPINNER, COLOR_CYAN).size(18),
        Space::with_width(Length::Fixed(10.0)),
        text("Mendeteksi perangkat penyimpanan...")
            .size(14)
            .color(TEXT_PRIMARY),
    ]
    .align_y(iced::Alignment::Center);

    let note = text("Memeriksa partisi NVMe, SATA SSD, HDD, dan filesystem mount...")
        .size(12)
        .color(TEXT_MUTED);

    let modal_box = container(
        column![
            logo_row,
            Space::with_height(Length::Fixed(12.0)),
            spinner_row,
            note,
        ]
        .spacing(12)
        .padding(28)
        .align_x(iced::Alignment::Center),
    )
    .style(card_style);

    container(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into()
}
