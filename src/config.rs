use serde::Deserialize;

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
}

impl Default for MusicPlayerConfig {
    fn default() -> Self {
        Self {
            initial_volume: 50,
            shuffle: Default::default(),
        }
    }
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
            Some(toml::from_str::<Self>(&config_string).unwrap())
        };
        opt().unwrap_or_default()
    }
}
