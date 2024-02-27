use directories::ProjectDirs;
use ratatui::style::{Color, Style};

/// Get directories for the project for config, cache, etc.
pub fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "ccgauche", "ytermusic")
}
/// Invert a style
pub fn invert(style: Style) -> Style {
    if style.bg.is_none() {
        return Style {
            fg: Some(color_contrast(style.fg.unwrap_or(Color::Reset))),
            bg: style.fg,
            add_modifier: style.add_modifier,
            sub_modifier: style.sub_modifier,
            underline_color: style.underline_color,
        };
    }
    Style {
        fg: style.bg,
        bg: style.fg,
        add_modifier: style.add_modifier,
        sub_modifier: style.sub_modifier,
        underline_color: style.underline_color,
    }
}

/// Returns a color with a high contrast to the input color (white or black)
pub fn color_contrast(color: Color) -> Color {
    match color {
        Color::Black => Color::White,
        Color::White => Color::Black,
        Color::Red => Color::White,
        Color::Green => Color::Black,
        Color::Yellow => Color::Black,
        Color::Blue => Color::White,
        Color::Magenta => Color::White,
        Color::Cyan => Color::Black,
        Color::Gray => Color::White,
        Color::DarkGray => Color::Black,
        Color::LightRed => Color::White,
        Color::LightGreen => Color::Black,
        Color::LightYellow => Color::Black,
        Color::LightBlue => Color::White,
        Color::LightMagenta => Color::White,
        Color::LightCyan => Color::Black,
        Color::Indexed(v) => {
            if v < 8 {
                Color::White
            } else {
                Color::Black
            }
        }
        Color::Rgb(r, g, b) => {
            if r as u32 + g as u32 + b as u32 > 382 {
                Color::Black
            } else {
                Color::White
            }
        }
        Color::Reset => Color::Black,
    }
}
