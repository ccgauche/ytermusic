use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct LrcLibResponse {
    plainLyrics: Option<String>,
}

#[derive(Clone)]
pub struct LyricsState {
    pub current_video_id: Option<String>,
    pub lyrics: Arc<Mutex<Option<String>>>,
    pub is_loading: Arc<Mutex<bool>>,
    pub scroll: u16,
}

impl Default for LyricsState {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsState {
    pub fn new() -> Self {
        Self {
            current_video_id: None,
            lyrics: Arc::new(Mutex::new(Some("No lyrics".to_string()))),
            is_loading: Arc::new(Mutex::new(false)),
            scroll: 0,
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn update(&mut self, video_id: Option<String>, title: &str, artist: &str) {
        if self.current_video_id == video_id {
            return;
        }
        self.current_video_id = video_id.clone();
        self.scroll = 0;

        if video_id.is_none() {
            *self.lyrics.lock().unwrap() = Some("No song playing".to_string());
            return;
        }

        *self.lyrics.lock().unwrap() = None;
        *self.is_loading.lock().unwrap() = true;

        let lyrics_clone = self.lyrics.clone();
        let loading_clone = self.is_loading.clone();

        // Clean up title (remove parenthesis content like "(Official Video)")
        let title_clean = title.split(" (").next().unwrap_or(title).to_string();
        let artist_clean = artist.to_string();

        tokio::spawn(async move {
            let url = format!(
                "https://lrclib.net/api/get?artist_name={}&track_name={}",
                urlencoding::encode(&artist_clean),
                urlencoding::encode(&title_clean)
            );

            if let Ok(res) = reqwest::get(&url).await {
                if let Ok(json) = res.json::<LrcLibResponse>().await {
                    if let Some(ly) = json.plainLyrics {
                        *lyrics_clone.lock().unwrap() = Some(ly);
                    } else {
                        *lyrics_clone.lock().unwrap() = Some("Lyrics not found".to_string());
                    }
                } else {
                    *lyrics_clone.lock().unwrap() = Some("Lyrics not found".to_string());
                }
            } else {
                *lyrics_clone.lock().unwrap() = Some("Network error fetching lyrics".to_string());
            }
            *loading_clone.lock().unwrap() = false;
        });
    }
}

impl Widget for &LyricsState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let is_loading = *self.is_loading.lock().unwrap();
        let lyrics_opt = self.lyrics.lock().unwrap().clone();

        let text = if is_loading {
            "Loading lyrics...".to_string()
        } else if let Some(ly) = lyrics_opt {
            ly
        } else {
            "No lyrics".to_string()
        };

        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Lyrics "))
            .style(Style::default().fg(Color::Rgb(226, 232, 240))) // Soft White
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .scroll((self.scroll, 0))
            .render(area, buf);
    }
}
