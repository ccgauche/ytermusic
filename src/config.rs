use serde::Deserialize;
use tui::style::{Color, Modifier, Style};

use crate::utils::get_project_dirs;

#[derive(Debug, Default, Deserialize)]
#[non_exhaustive]
pub struct GlobalConfig {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct MusicPlayerConfig {
    /// Initial volume of the player, in percent.
    /// Default value is 50, clamped at 100.
    #[serde(default = "default_volume")]
    pub initial_volume: u8,
    /// Whether to shuffle playlists before playing
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default = "default_paused_style", with = "StyleDef")]
    pub gauge_paused_style: Style,
    #[serde(default = "default_playing_style", with = "StyleDef")]
    pub gauge_playing_style: Style,
    #[serde(default = "default_nomusic_style", with = "StyleDef")]
    pub gauge_nomusic_style: Style,
    #[serde(default = "default_paused_style", with = "StyleDef")]
    pub text_paused_style: Style,
    #[serde(default = "default_playing_style", with = "StyleDef")]
    pub text_playing_style: Style,
    #[serde(default = "default_nomusic_style", with = "StyleDef")]
    pub text_next_style: Style,
    #[serde(default = "default_nomusic_style", with = "StyleDef")]
    pub text_previous_style: Style,
    #[serde(default = "default_downloading_style", with = "StyleDef")]
    pub text_downloading_style: Style,
}

#[derive(Debug, Deserialize)]
#[serde(remote = "Style")]
struct StyleDef {
    #[serde(default)]
    fg: Option<Color>,
    #[serde(default)]
    bg: Option<Color>,
    #[serde(default = "Modifier::empty")]
    add_modifier: Modifier,
    #[serde(default = "Modifier::empty")]
    sub_modifier: Modifier,
}

impl Default for MusicPlayerConfig {
    fn default() -> Self {
        Self {
            initial_volume: default_volume(),
            shuffle: Default::default(),
            gauge_paused_style: default_paused_style(),
            gauge_playing_style: default_playing_style(),
            gauge_nomusic_style: default_nomusic_style(),
            text_paused_style: default_paused_style(),
            text_playing_style: default_playing_style(),
            text_next_style: default_nomusic_style(),
            text_previous_style: default_nomusic_style(),
            text_downloading_style: default_downloading_style(),
        }
    }
}

fn default_paused_style() -> Style {
    Style::default().fg(Color::Yellow).bg(Color::Black)
}

fn default_playing_style() -> Style {
    Style::default().fg(Color::Green).bg(Color::Black)
}

fn default_nomusic_style() -> Style {
    Style::default().fg(Color::White).bg(Color::Black)
}

fn default_downloading_style() -> Style {
    Style::default().fg(Color::Blue).bg(Color::Black)
}

fn default_volume() -> u8 {
    50
}

#[derive(Debug, Default, Deserialize)]
#[non_exhaustive]
pub struct PlaylistConfig {}

#[derive(Debug, Default, Deserialize)]
#[non_exhaustive]
pub struct SearchConfig {}

#[allow(unused)]
#[derive(Debug, Default, Deserialize)]
#[non_exhaustive]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub player: MusicPlayerConfig,
    #[serde(default)]
    pub playlist: PlaylistConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

impl Config {
    pub fn new() -> Self {
        // TODO handle errors
        let opt = || {
            let project_dirs = get_project_dirs()?;
            let config_path = project_dirs.config_dir().join("config.toml");
            let config_string = std::fs::read_to_string(config_path).ok()?;
            toml::from_str::<Self>(&config_string).ok()
        };
        opt().unwrap_or_default()
    }
}
