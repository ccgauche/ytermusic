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
    fn on_mouse_press(
        &mut self,
        _: crossterm::event::MouseEvent,
        _: &Rect,
    ) -> super::EventResponse {
        super::EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> super::EventResponse {
        if KeyCode::Esc == key.code {
            return EventResponse::Message(vec![ManagerMessage::Quit]);
        }
        if KeyCode::Enter == key.code || KeyCode::Char(' ') == key.code {
            return EventResponse::Message(vec![ManagerMessage::PassTo(
                Screens::MusicPlayer,
                Box::new(ManagerMessage::RestartPlayer),
            )]);
        }

        EventResponse::None
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

    fn handle_global_message(&mut self, _: super::ManagerMessage) -> super::EventResponse {
        EventResponse::None
    }

    fn close(&mut self, _: Screens) -> super::EventResponse {
        EventResponse::None
    }

    fn open(&mut self) -> super::EventResponse {
        EventResponse::None
    }
}
