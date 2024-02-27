use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::consts::CONFIG;

use super::{EventResponse, ManagerMessage, Screen, Screens};

// Audio device not connected!
pub struct DeviceLost(pub Vec<String>, pub Option<ManagerMessage>);

impl Screen for DeviceLost {
    fn on_mouse_press(&mut self, _: crossterm::event::MouseEvent, _: &Rect) -> EventResponse {
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(m) = self.1.take() {
                    EventResponse::Message(vec![m])
                } else {
                    ManagerMessage::RestartPlayer
                        .pass_to(Screens::MusicPlayer)
                        .event()
                }
            }
            KeyCode::Esc => ManagerMessage::Quit.event(),
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        frame.render_widget(
            Paragraph::new(format!(
                "{}\nPress [Enter] or [Space] to retry.\nOr [Esc] to exit",
                self.0.join("\n")
            ))
            .style(CONFIG.player.text_error_style)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(CONFIG.player.text_next_style)
                    .title(" Error ")
                    .border_type(BorderType::Plain),
            ),
            frame.size(),
        );
    }

    fn handle_global_message(&mut self, m: ManagerMessage) -> EventResponse {
        match m {
            ManagerMessage::Error(a, m) => {
                self.1 = *m;
                self.0.push(a);
                EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::DeviceLost)])
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        self.0.clear();
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
