use directories::ProjectDirs;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use unicode_bidi::{BidiInfo, Level};

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

/// Reorder a string using the Unicode Bidirectional Algorithm.
/// This ensures RTL text (e.g. Hebrew, Arabic) displays correctly in the terminal.
pub fn to_bidi_string(s: &str) -> String {
    let bidi_info = BidiInfo::new(s, None);
    if let Some(para) = bidi_info.paragraphs.first() {
        if para.level != Level::ltr() {
            return bidi_info.reorder_line(para, para.range.clone()).to_string();
        }
        // Check if any character has RTL level even in an LTR paragraph
        let start = para.range.start;
        let end = para.range.end;
        if bidi_info.levels[start..end]
            .iter()
            .any(|l| *l != Level::ltr())
        {
            return bidi_info.reorder_line(para, para.range.clone()).to_string();
        }
    }
    s.to_string()
}

/// Interpolates between two colors
pub fn interpolate_color(color1: Color, color2: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (r1, g1, b1) = match color1 {
        Color::Rgb(r, g, b) => (r as f64, g as f64, b as f64),
        _ => (0.0, 0.0, 0.0), // Simplified, assuming we always pass RGB
    };
    let (r2, g2, b2) = match color2 {
        Color::Rgb(r, g, b) => (r as f64, g as f64, b as f64),
        _ => (0.0, 0.0, 0.0),
    };

    let r = (r1 * (1.0 - t) + r2 * t) as u8;
    let g = (g1 * (1.0 - t) + g2 * t) as u8;
    let b = (b1 * (1.0 - t) + b2 * t) as u8;

    Color::Rgb(r, g, b)
}

/// Fills a region of the buffer with a vertical gradient background
pub fn fill_gradient_bg(buf: &mut Buffer, area: Rect, top_color: Color, bottom_color: Color) {
    if area.height == 0 {
        return;
    }
    for y in area.top()..area.bottom() {
        let t = (y - area.top()) as f64 / area.height as f64;
        let color = interpolate_color(top_color, bottom_color, t);
        for x in area.left()..area.right() {
            buf.get_mut(x, y).set_bg(color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_midpoint_between_rgb_colors() {
        assert_eq!(
            interpolate_color(Color::Rgb(0, 10, 20), Color::Rgb(100, 110, 120), 0.5,),
            Color::Rgb(50, 60, 70)
        );
    }

    #[test]
    fn fills_each_row_with_a_gradient_color() {
        let area = Rect::new(0, 0, 2, 3);
        let mut buffer = Buffer::empty(area);

        fill_gradient_bg(
            &mut buffer,
            area,
            Color::Rgb(0, 0, 0),
            Color::Rgb(90, 90, 90),
        );

        assert_eq!(buffer.get(0, 0).bg, buffer.get(1, 0).bg);
        assert_ne!(buffer.get(0, 0).bg, buffer.get(0, 2).bg);
    }
}
