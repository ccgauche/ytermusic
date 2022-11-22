use tui::style::{Color, Style};

use crate::consts::CONFIG;

#[derive(PartialEq, Debug, Clone)]
pub enum AppStatus {
    Paused,
    Playing,
    NoMusic,
}

impl AppStatus {
    pub fn style(&self) -> Style {
        match self {
            AppStatus::Paused => CONFIG.player.paused_style,
            AppStatus::Playing => CONFIG.player.playing_style,
            AppStatus::NoMusic => CONFIG.player.nomusic_style,
        }
    }
}
