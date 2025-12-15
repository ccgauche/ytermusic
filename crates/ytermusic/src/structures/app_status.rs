use ratatui::style::{Modifier, Style};

use crate::consts::CONFIG;

#[derive(PartialEq, Debug, Clone)]
pub enum AppStatus {
    Paused,
    Playing,
    NoMusic,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MusicDownloadStatus {
    NotDownloaded,
    Downloaded,
    Downloading(usize),
    DownloadFailed,
}

impl MusicDownloadStatus {
    pub fn character(&self, playing: Option<bool>) -> String {
        match self {
            Self::NotDownloaded => {
                if let Some(e) = playing {
                    if e {
                        '▶'
                    } else {
                        '⏸'
                    }
                } else {
                    ' '
                }
            }
            Self::Downloaded => ' ',
            Self::Downloading(progress) => return format!("⭳ [{:02}%]", progress),
            Self::DownloadFailed => '⚠',
        }
        .into()
    }
    pub fn style(&self, playing: Option<bool>) -> Style {
        let k = match self {
            Self::NotDownloaded => CONFIG.player.text_waiting_style,
            Self::Downloaded => {
                if let Some(e) = playing {
                    if e {
                        CONFIG.player.text_playing_style
                    } else {
                        CONFIG.player.text_paused_style
                    }
                } else {
                    CONFIG.player.text_next_style
                }
            }
            Self::Downloading(_) => CONFIG.player.text_downloading_style,
            Self::DownloadFailed => CONFIG.player.text_error_style,
        };
        if playing.is_some() {
            k.add_modifier(Modifier::BOLD)
        } else {
            k
        }
    }
}

impl AppStatus {
    pub fn style(&self) -> Style {
        match self {
            AppStatus::Paused => CONFIG.player.gauge_paused_style,
            AppStatus::Playing => CONFIG.player.gauge_playing_style,
            AppStatus::NoMusic => CONFIG.player.gauge_nomusic_style,
        }
    }
}
