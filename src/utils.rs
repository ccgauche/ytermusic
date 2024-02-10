use std::path::PathBuf;
use std::str::FromStr;
use directories::ProjectDirs;
use ratatui::style::Style;

/// Get directories for the project for config, cache, etc.
pub fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "ccgauche", "ytermusic")
}

/// Locate the headers.txt file:
pub fn locate_headers_file() -> Option<PathBuf> {

    // Locate the headers.txt file:
    let header_paths: [PathBuf; 2] = [
        PathBuf::from_str("./headers.txt").unwrap(),
        get_project_dirs()?.config_dir().join("headers.txt"),
    ];

    // Return the first path that exists, if any:
    header_paths.iter().find(|path| path.exists()).cloned()
}

/// Invert a style
pub fn invert(style: Style) -> Style {
    Style {
        fg: style.bg,
        bg: style.fg,
        add_modifier: style.add_modifier,
        sub_modifier: style.sub_modifier,
        underline_color: style.underline_color,
    }
}
