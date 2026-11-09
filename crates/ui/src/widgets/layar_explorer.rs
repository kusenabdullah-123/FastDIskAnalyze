use crate::icons::{
    icon, icon_colored, ICON_ARROW_LEFT, ICON_BOLT, ICON_DATABASE, ICON_GLOBE, ICON_HDD, ICON_USB,
};
use crate::theme::{
    chip_style, secondary_button, COLOR_CYAN, COLOR_PRIMARY, COLOR_SUCCESS, COLOR_WARNING,
    TEXT_PRIMARY,
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
) -> Element<'a, Message> {
    // 1. Toolbar Atas Minimalis & Elegan
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

    let toolbar = row![
        back_btn,
        Space::with_width(Length::Fixed(8.0)),
        drive_badge,
        Space::with_width(Length::Fill),
    ]
    .align_y(iced::Alignment::Center);

    // 2. Breadcrumb Bar
    let breadcrumb = breadcrumb_bar(
        model,
        state.current_dir,
        on_navigate,
        on_navigate_up,
    );

    // 3. Tabel Virtual (2 Kolom: NAMA dan UKURAN)
    let table = virtual_table_widget(
        model,
        state,
        on_toggle_sort,
        on_navigate,
        on_select_node,
        on_scroll_table,
    );

    // 4. Status Bar
    let status = status_bar_widget(
        state,
        Some(model),
        None,
        Some(drive.medium),
    );

    column![
        toolbar,
        breadcrumb,
        table,
        status,
    ]
    .spacing(8)
    .padding(14)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
