use directories::ProjectDirs;
use tui::style::Style;

// We don't know if current is bigger than 2 so we can't clamp or it can panic.
#[allow(clippy::manual_clamp)]
pub fn get_before(lines: usize, current: usize, size: usize) -> usize {
    // Remove the margin
    ((lines - 5).saturating_add(current).saturating_sub(size))
        .max(2)
        .min(current)
}

/// Get directories for the project for config, cache, etc.
pub fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "ccgauche", "ytermusic")
}

pub fn invert(style: Style) -> Style {
    Style {
        fg: style.bg,
        bg: style.fg,
        add_modifier: style.add_modifier,
        sub_modifier: style.sub_modifier,
    }
}
