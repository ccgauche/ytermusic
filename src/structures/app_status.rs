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
            Self::Downloading(_) => Style::default().fg(Color::Cyan).bg(Color::Black),
            Self::DownloadFailed => Style::default().fg(Color::Red).bg(Color::Black),
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
