use crate::icons::{
    icon, icon_colored, ICON_ARROW_LEFT, ICON_BOLT, ICON_DATABASE, ICON_GLOBE, ICON_HDD,
    ICON_REFRESH, ICON_TRASH, ICON_USB,
};
use crate::theme::{
    card_style, chip_style, danger_button, ghost_button, secondary_button, COLOR_CYAN,
    COLOR_DANGER, COLOR_PRIMARY, COLOR_SUCCESS, COLOR_WARNING, TEXT_PRIMARY,
};
use crate::view_state::{SortColumn, ViewState};
use crate::widgets::breadcrumb::breadcrumb_bar;
use crate::widgets::status_bar::status_bar_widget;
use crate::widgets::table::virtual_table_widget;
use fast_disk_analyze_core::{IdEntri, PohonSistemBerkas};
use fast_disk_analyze_platform::{InformasiDrive, MediumKind};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length};

pub fn view_explorer<'a, Message: 'a + Clone + 'static>(
    drive: &'a InformasiDrive,
    model: &'a PohonSistemBerkas,
    state: &ViewState,
    on_back_to_drives: Message,
    on_navigate: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_navigate_up: Message,
    on_select_node: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_toggle_sort: impl Fn(SortColumn) -> Message + 'static + Copy,
    on_scroll_table: impl Fn(iced::widget::scrollable::Viewport) -> Message + 'static + Copy,
    on_rescan_subtree: Message,
    on_delete_trash: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_right_click: impl Fn(IdEntri) -> Message + 'static + Copy,
    on_close_context_menu: Message,
) -> Element<'a, Message> {
    // 1. Toolbar Atas Minimalis (Hanya Ganti Drive, Badge Drive, dan Refresh)
    let back_btn = button(
        row![
            icon(ICON_ARROW_LEFT).size(12),
            Space::with_width(Length::Fixed(6.0)),
            text("Ganti Drive").size(12),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 14])
    .style(secondary_button)
    .on_press(on_back_to_drives);

    let (drive_icon, drive_color) = match drive.medium {
        MediumKind::NvmeSsd => (ICON_BOLT, COLOR_CYAN),
        MediumKind::SataSsd => (ICON_BOLT, COLOR_PRIMARY),
        MediumKind::Hdd => (ICON_HDD, COLOR_WARNING),
        MediumKind::Removable => (ICON_USB, COLOR_SUCCESS),
        MediumKind::Network => (ICON_GLOBE, COLOR_PRIMARY),
        MediumKind::Unknown => (ICON_DATABASE, COLOR_CYAN),
    };

    let drive_badge = container(
        row![
            icon_colored(drive_icon, drive_color).size(12),
            Space::with_width(Length::Fixed(6.0)),
            text(&drive.label).size(12).color(TEXT_PRIMARY),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([5, 12])
    .style(chip_style);

    let refresh_btn = button(
        row![
            icon(ICON_REFRESH).size(13),
            Space::with_width(Length::Fixed(6.0)),
            text("Refresh").size(12),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 12])
    .style(ghost_button)
    .on_press(on_rescan_subtree);

    let toolbar = row![
        back_btn,
        Space::with_width(Length::Fixed(8.0)),
        drive_badge,
        Space::with_width(Length::Fill),
        refresh_btn,
    ]
    .align_y(iced::Alignment::Center);

    // 2. Breadcrumb Bar
    let breadcrumb = breadcrumb_bar(
        model,
        state.current_dir,
        on_navigate,
        on_navigate_up,
    );

    // Context Action Banner (muncul saat baris di-klik kanan)
    let maybe_context_banner: Option<Element<'a, Message>> = state.context_menu_target.map(|target_id| {
        let nama = model.nama(target_id);
        let is_dir = model.adalah_direktori(target_id);
        let item_label = if is_dir { "Folder" } else { "Berkas" };

        container(
            row![
                icon_colored(ICON_TRASH, COLOR_DANGER).size(13),
                Space::with_width(Length::Fixed(8.0)),
                text(format!("{item_label} \"{nama}\":")).size(12).color(TEXT_PRIMARY),
                Space::with_width(Length::Fill),
                button(text("Batal").size(11))
                    .padding([4, 10])
                    .style(ghost_button)
                    .on_press(on_close_context_menu),
                Space::with_width(Length::Fixed(6.0)),
                button(
                    row![
                        icon(ICON_TRASH).size(11),
                        Space::with_width(Length::Fixed(4.0)),
                        text("Pindahkan ke Trash").size(11),
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .padding([4, 12])
                .style(danger_button)
                .on_press(on_delete_trash(target_id)),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([6, 12])
        .style(card_style)
        .width(Length::Fill)
        .into()
    });

    // 3. Tabel Virtual (NAMA, UKURAN, dan tombol [ 🗑️ ] di sebelah kanan kapasitas)
    let table = virtual_table_widget(
        model,
        state,
        on_toggle_sort,
        on_navigate,
        on_select_node,
        on_scroll_table,
        on_delete_trash,
        on_right_click,
    );

    // 4. Status Bar
    let status = status_bar_widget(
        state,
        Some(model),
        None,
        Some(drive.medium),
    );

    let mut layout = column![
        toolbar,
        breadcrumb,
    ]
    .spacing(8);

    if let Some(banner) = maybe_context_banner {
        layout = layout.push(banner);
    }

    layout = layout.push(table).push(status);

    layout
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
