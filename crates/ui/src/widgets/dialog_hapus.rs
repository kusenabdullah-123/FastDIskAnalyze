use crate::icons::{icon_colored, ICON_TRASH};
use crate::theme::{
    card_style, danger_button, ghost_button, COLOR_DANGER, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::widgets::size_bar::format_bytes;
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length};

pub fn view_dialog_hapus<'a, Message: 'a + Clone + 'static>(
    pohon: &'a PohonSistemBerkas,
    target_id: IdEntri,
    on_confirm: Message,
    on_cancel: Message,
) -> Element<'a, Message> {
    let nama = pohon.nama(target_id);
    let path = pohon.fs_path_verbatim(target_id);
    let size = pohon.gudang.ambil(target_id).ukuran_blok;
    let is_dir = pohon.adalah_direktori(target_id);

    let item_type = if is_dir { "folder" } else { "berkas" };
    let header_icon = icon_colored(ICON_TRASH, COLOR_DANGER).size(20);
    let title_text = text(format!("Yakin menghapus {item_type}?"))
        .size(16)
        .color(TEXT_PRIMARY);

    let title_row = row![
        header_icon,
        Space::with_width(Length::Fixed(8.0)),
        title_text,
    ]
    .align_y(iced::Alignment::Center);

    let desc = text(format!(
        "Apakah Anda yakin ingin memindahkan {item_type} \"{nama}\" ke Tempat Sampah?"
    ))
    .size(13)
    .color(TEXT_PRIMARY);

    let info_box = column![
        text(format!("Lokasi: {}", path.0.display()))
            .size(11)
            .color(TEXT_MUTED),
        text(format!("Ukuran: {}", format_bytes(size)))
            .size(12)
            .color(TEXT_SECONDARY),
    ]
    .spacing(3);

    // Dua tombol: "Tidak" dan "Iya"
    let no_btn = button(text("Tidak").size(12))
        .padding([7, 20])
        .style(ghost_button)
        .on_press(on_cancel);

    let yes_btn = button(
        row![
            icon_colored(ICON_TRASH, iced::Color::WHITE).size(12),
            Space::with_width(Length::Fixed(6.0)),
            text("Iya").size(12),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([7, 20])
    .style(danger_button)
    .on_press(on_confirm);

    let action_row = row![
        Space::with_width(Length::Fill),
        no_btn,
        Space::with_width(Length::Fixed(10.0)),
        yes_btn,
    ]
    .align_y(iced::Alignment::Center);

    let dialog_box = container(
        column![
            title_row,
            Space::with_height(Length::Fixed(12.0)),
            desc,
            Space::with_height(Length::Fixed(8.0)),
            info_box,
            Space::with_height(Length::Fixed(20.0)),
            action_row,
        ]
        .spacing(4),
    )
    .padding(20)
    .width(Length::Fixed(460.0))
    .style(card_style);

    // Overlay modal di tengah layar
    container(dialog_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.65))),
            ..Default::default()
        })
        .into()
}
