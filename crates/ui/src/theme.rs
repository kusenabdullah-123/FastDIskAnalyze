use iced::widget::{button, container, progress_bar, text_input};
use iced::{Background, Border, Color, Theme};

// =========================================================================
// PALET WARNA MODERN (Slate Dark Theme / Indigo & Emerald Accents)
// =========================================================================
pub const BG_APP: Color = Color::from_rgb(0.06, 0.09, 0.15); // #0f172a (Deep Slate)
pub const BG_NAVBAR: Color = Color::from_rgb(0.08, 0.11, 0.18); // #141c2e
pub const BG_CARD: Color = Color::from_rgb(0.12, 0.16, 0.24); // #1e293b (Surface Card)
pub const BG_CARD_HOVER: Color = Color::from_rgb(0.16, 0.22, 0.32); // #293548
pub const BG_INPUT: Color = Color::from_rgb(0.08, 0.11, 0.18); // #141c2e
pub const BG_CHIP: Color = Color::from_rgb(0.15, 0.20, 0.29); // #26334a

pub const BORDER_SUBTLE: Color = Color::from_rgb(0.20, 0.27, 0.38); // #334155
pub const BORDER_FOCUS: Color = Color::from_rgb(0.38, 0.53, 0.95); // #60a5fa

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.96, 0.97, 0.99); // #f8fafc
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.58, 0.64, 0.72); // #94a3b8
pub const TEXT_MUTED: Color = Color::from_rgb(0.39, 0.45, 0.55); // #64748b

// Warna Aksen Semantik
pub const COLOR_PRIMARY: Color = Color::from_rgb(0.25, 0.47, 0.95); // #3b82f6 (Blue)
pub const COLOR_PRIMARY_HOVER: Color = Color::from_rgb(0.35, 0.55, 1.0); // #60a5fa
pub const COLOR_SUCCESS: Color = Color::from_rgb(0.06, 0.78, 0.57); // #10b981 (Emerald)
pub const COLOR_WARNING: Color = Color::from_rgb(0.96, 0.62, 0.04); // #f59e0b (Amber)
pub const COLOR_DANGER: Color = Color::from_rgb(0.94, 0.27, 0.34); // #f43f5e (Rose)
pub const COLOR_DANGER_HOVER: Color = Color::from_rgb(1.0, 0.35, 0.42);
pub const COLOR_FOLDER: Color = Color::from_rgb(0.98, 0.75, 0.18); // #fbbf24 (Golden Amber)
pub const COLOR_CYAN: Color = Color::from_rgb(0.06, 0.73, 0.85); // #06b6d4 (Cyan)

// =========================================================================
// STYLES CONTAINER
// =========================================================================

pub fn app_background(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_APP)),
        border: Border::default(),
        shadow: Default::default(),
    }
}

pub fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_CARD)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn card_hover_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_CARD_HOVER)),
        border: Border {
            color: COLOR_PRIMARY,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn boxed_list_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_CARD)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn icon_badge_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_PRIMARY),
        background: Some(Background::Color(BG_NAVBAR)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn divider_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BORDER_SUBTLE)),
        border: Border::default(),
        shadow: Default::default(),
        text_color: None,
    }
}

pub fn drive_row_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(BG_CARD_HOVER)),
        button::Status::Pressed => Some(Background::Color(BG_NAVBAR)),
        _ => None,
    };

    button::Style {
        background: bg,
        text_color: TEXT_PRIMARY,
        border: Border::default(),
        shadow: Default::default(),
    }
}

pub fn chip_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SECONDARY),
        background: Some(Background::Color(BG_CHIP)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn table_header_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SECONDARY),
        background: Some(Background::Color(BG_NAVBAR)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn table_row_container(is_selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        if is_selected {
            container::Style {
                text_color: Some(TEXT_PRIMARY),
                background: Some(Background::Color(Color::from_rgba(0.25, 0.47, 0.95, 0.25))),
                border: Border {
                    color: COLOR_PRIMARY,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: Default::default(),
            }
        } else {
            container::Style {
                text_color: Some(TEXT_PRIMARY),
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 6.0.into(),
                },
                shadow: Default::default(),
            }
        }
    }
}

// =========================================================================
// STYLES BUTTON
// =========================================================================

pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => COLOR_PRIMARY_HOVER,
        button::Status::Pressed => COLOR_PRIMARY,
        _ => COLOR_PRIMARY,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, border_col) = match status {
        button::Status::Hovered => (BG_CARD_HOVER, BORDER_FOCUS),
        button::Status::Pressed => (BG_CARD, BORDER_FOCUS),
        _ => (BG_CARD, BORDER_SUBTLE),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: border_col,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text_col) = match status {
        button::Status::Hovered => (Some(Background::Color(BG_CARD_HOVER)), TEXT_PRIMARY),
        button::Status::Pressed => (Some(Background::Color(BG_CARD)), TEXT_PRIMARY),
        _ => (None, TEXT_SECONDARY),
    };

    button::Style {
        background: bg,
        text_color: text_col,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => COLOR_DANGER_HOVER,
        button::Status::Pressed => COLOR_DANGER,
        _ => COLOR_DANGER,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn header_sort_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(BG_CARD_HOVER)),
        button::Status::Pressed => Some(Background::Color(BG_CARD)),
        _ => None,
    };

    button::Style {
        background: bg,
        text_color: match status {
            button::Status::Hovered => TEXT_PRIMARY,
            _ => TEXT_SECONDARY,
        },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Default::default(),
    }
}

pub fn table_row_button(is_selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, border_col) = if is_selected {
            (Color::from_rgba(0.25, 0.47, 0.95, 0.25), COLOR_PRIMARY)
        } else {
            match status {
                button::Status::Hovered => (BG_CARD_HOVER, BORDER_SUBTLE),
                button::Status::Pressed => (BG_CARD, BORDER_SUBTLE),
                _ => (Color::TRANSPARENT, Color::TRANSPARENT),
            }
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: border_col,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: Default::default(),
        }
    }
}

// =========================================================================
// STYLES TEXT INPUT
// =========================================================================

pub fn search_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused => BORDER_FOCUS,
        text_input::Status::Hovered => COLOR_PRIMARY,
        _ => BORDER_SUBTLE,
    };

    text_input::Style {
        background: Background::Color(BG_INPUT),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: TEXT_MUTED,
        placeholder: TEXT_MUTED,
        value: TEXT_PRIMARY,
        selection: Color::from_rgba(0.25, 0.47, 0.95, 0.35),
    }
}

// =========================================================================
// STYLES PROGRESS BAR
// =========================================================================

pub fn dynamic_progress_bar_style(pct: f32) -> impl Fn(&Theme) -> progress_bar::Style {
    let bar_color = if pct > 0.90 {
        COLOR_DANGER
    } else if pct > 0.75 {
        COLOR_WARNING
    } else {
        COLOR_SUCCESS
    };

    move |_theme| progress_bar::Style {
        background: Background::Color(Color::from_rgb(0.08, 0.11, 0.18)),
        bar: Background::Color(bar_color),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 5.0.into(),
        },
    }
}
