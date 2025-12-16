use common_structs::{AppStatus, MusicDownloadStatus};
use ratatui::style::{Modifier, Style};

pub trait MusicDownloadStatusExt {
    fn style(&self, playing: Option<bool>) -> Style;
}

pub trait AppStatusExt {
    fn style(&self) -> Style;
}

use crate::consts::CONFIG;

impl MusicDownloadStatusExt for MusicDownloadStatus {
    fn style(&self, playing: Option<bool>) -> Style {
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

impl AppStatusExt for AppStatus {
    fn style(&self) -> Style {
        match self {
            Self::Paused => CONFIG.player.gauge_paused_style,
            Self::Playing => CONFIG.player.gauge_playing_style,
            Self::NoMusic => CONFIG.player.gauge_nomusic_style,
        }
    }
}
