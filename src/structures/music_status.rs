use tui::style::Color;

#[derive(PartialEq, Debug, Clone)]
pub enum MusicStatus {
    Playing,
    Paused,
    Previous,
    Next,
    Downloading,
}

impl MusicStatus {
    pub fn character(&self) -> char {
        match self {
            MusicStatus::Playing => '▶',
            MusicStatus::Paused => '⏸',
            MusicStatus::Previous => ' ',
            MusicStatus::Next => ' ',
            MusicStatus::Downloading => '⭳',
        }
    }

    pub fn colors(&self) -> (Color, Color) {
        match self {
            MusicStatus::Playing => (Color::Green, Color::Black),
            MusicStatus::Paused => (Color::Yellow, Color::Black),
            MusicStatus::Previous => (Color::White, Color::Black),
            MusicStatus::Next => (Color::White, Color::Black),
            MusicStatus::Downloading => (Color::Blue, Color::Black),
        }
    }
}
