use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use flume::Sender;
use tui::{
    layout::Rect,
    style::{Color, Style},
    Frame,
};
use ytpapi::Video;

use crate::structures::sound_action::SoundAction;

use super::{
    item_list::{ListItem, ListItemAction},
    EventResponse, ManagerMessage, Screen, Screens,
};

#[derive(Clone)]
pub struct PlayListAction(usize);

impl ListItemAction for PlayListAction {
    fn render_style(&self, _: &str, selected: bool) -> Style {
        if selected {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        }
    }
}

// Audio device not connected!
pub struct PlaylistView {
    pub items: ListItem<PlayListAction>,
    pub videos: Vec<Video>,
    pub sender: Arc<Sender<SoundAction>>,
}

impl Screen for PlaylistView {
    fn on_mouse_press(&mut self, e: crossterm::event::MouseEvent, r: &Rect) -> EventResponse {
        if let Some(PlayListAction(v)) = self.items.on_mouse_press(e, r) {
            self.sender
                .send(SoundAction::ReplaceQueue(
                    self.videos.iter().skip(v).cloned().collect(),
                ))
                .unwrap();
            EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::MusicPlayer)])
        } else {
            EventResponse::None
        }
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        if let Some(PlayListAction(v)) = self.items.on_key_press(key) {
            self.sender
                .send(SoundAction::ReplaceQueue(
                    self.videos.iter().skip(*v).cloned().collect(),
                ))
                .unwrap();
            return EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::MusicPlayer)]);
        }
        match key.code {
            KeyCode::Esc => ManagerMessage::ChangeState(Screens::MusicPlayer).event(),
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, frame: &mut Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        frame.render_widget(&self.items, frame.size());
    }

    fn handle_global_message(&mut self, m: ManagerMessage) -> EventResponse {
        match m {
            ManagerMessage::Inspect(a, m) => {
                self.items.set_title(a);
                self.items.update(
                    m.iter()
                        .enumerate()
                        .map(|(i, m)| (m.to_string(), PlayListAction(i)))
                        .collect(),
                    0,
                );
                self.videos = m;

                EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::PlaylistViewer)])
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
