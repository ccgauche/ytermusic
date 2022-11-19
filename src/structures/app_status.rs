use tui::style::Color;

#[derive(PartialEq, Debug, Clone)]
pub enum AppStatus {
    Paused,
    Playing,
    NoMusic,
}

impl AppStatus {
    pub fn colors(&self) -> (Color, Color) {
        match self {
            AppStatus::Paused => (Color::Yellow, Color::Black),
            AppStatus::Playing => (Color::Green, Color::Black),
            AppStatus::NoMusic => (Color::White, Color::Black),
        }
    }
}
