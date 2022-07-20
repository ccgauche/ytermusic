use crossterm::event::{KeyCode, KeyEvent};
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::{EventResponse, ManagerMessage, Screen, Screens};

pub struct DeviceLost;

impl Screen for DeviceLost {
    fn on_mouse_press(&mut self, _: crossterm::event::MouseEvent, _: &Rect) -> EventResponse {
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => ManagerMessage::RestartPlayer
                .pass_to(Screens::MusicPlayer)
                .event(),
            KeyCode::Esc => ManagerMessage::Quit.event(),
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        frame.render_widget(
            Paragraph::new(
                "Audio device not connected!\nPress [Enter] or [Space] to retry.\nOr [Esc] to exit",
            )
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title(" Error ")
                    .border_type(BorderType::Plain),
            ),
            frame.size(),
        );
    }

    fn handle_global_message(&mut self, _: ManagerMessage) -> EventResponse {
        EventResponse::None
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
