use tui::style::{Color, Modifier, Style};

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
}

impl MusicDownloadStatus {
    pub fn character(&self, playing: Option<bool>) -> char {
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
            Self::Downloading(_) => '⭳',
        }
    }
    pub fn style(&self, playing: Option<bool>) -> Style {
        let k = match self {
            Self::NotDownloaded => Style::default().fg(Color::Gray).bg(Color::Black),
            Self::Downloaded => {
                if let Some(e) = playing {
                    if e {
                        Style::default().fg(Color::Green).bg(Color::Black)
                    } else {
                        Style::default().fg(Color::Yellow).bg(Color::Black)
                    }
                } else {
                    Style::default().fg(Color::White).bg(Color::Black)
                }
            }
            Self::Downloading(_) => Style::default().fg(Color::Blue).bg(Color::Black),
        };
        if let Some(e) = playing {
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
