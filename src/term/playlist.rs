use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use flume::Sender;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use ytpapi::Video;

use crate::{systems::download, SoundAction};

use super::{rect_contains, relative_pos, EventResponse, ManagerMessage, Screen, Screens};

pub struct Chooser {
    pub selected: usize,
    pub items: Vec<(String, Vec<Video>)>,
    pub action_sender: Arc<Sender<SoundAction>>,
}
impl Screen for Chooser {
    fn on_mouse_press(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
        frame_data: &Rect,
    ) -> EventResponse {
        if let MouseEventKind::Down(_) = mouse_event.kind {
            let x = mouse_event.column;
            let y = mouse_event.row;
            if rect_contains(frame_data, x, y, 1) {
                let (_, y) = relative_pos(frame_data, x, y, 1);
                let y = if self.selected == 0 {
                    y
                } else {
                    y + self.selected as u16 - 1
                };
                if self.items.len() > y as usize {
                    self.selected = y as usize;
                    return self.on_key_press(
                        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                        frame_data,
                    );
                }
            }
        }
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => return ManagerMessage::ChangeState(Screens::MusicPlayer).event(),
            KeyCode::Char('f') => return ManagerMessage::ChangeState(Screens::Search).event(),
            KeyCode::Enter => {
                if let Some(a) = &self.items.get(self.selected) {
                    if a.0 != "Local musics" {
                        std::fs::write(
                            "data/last-playlist.json",
                            serde_json::to_string(&a).unwrap(),
                        )
                        .unwrap();
                    }
                    self.action_sender.send(SoundAction::Cleanup).unwrap();
                    download::clean(self.action_sender.clone());
                    for video in self.items.get(self.selected).unwrap().1.iter() {
                        download::add(video.clone(), &self.action_sender);
                    }
                }
                return EventResponse::Message(vec![ManagerMessage::ChangeState(
                    Screens::MusicPlayer,
                )]);
            }
            KeyCode::Char('+') | KeyCode::Up => self.selected(self.selected as isize - 1),
            KeyCode::Char('-') | KeyCode::Down => self.selected(self.selected as isize + 1),
            _ => {}
        }
        EventResponse::None
    }

    fn render(&mut self, frame: &mut Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        frame.render_stateful_widget(
            List::new(
                self.items
                    .iter()
                    .enumerate()
                    .skip(self.selected.saturating_sub(1))
                    .map(|(index, i)| {
                        ListItem::new(i.0.as_str()).style(
                            Style::default()
                                .fg(if index == self.selected {
                                    Color::Black
                                } else {
                                    Color::White
                                })
                                .bg(if index != self.selected {
                                    Color::Black
                                } else {
                                    Color::White
                                }),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Select the playlist to play "),
            ),
            frame.size(),
            &mut ListState::default(),
        );
    }

    fn handle_global_message(&mut self, message: super::ManagerMessage) -> EventResponse {
        if let ManagerMessage::AddElementToChooser(a) = message {
            self.add_element(a);
        }
        EventResponse::None
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
impl Chooser {
    fn selected(&mut self, selected: isize) {
        if selected < 0 {
            self.selected = self.items.len() - 1;
        } else if selected >= self.items.len() as isize {
            self.selected = 0;
        } else {
            self.selected = selected as usize;
        }
    }
    fn add_element(&mut self, element: (String, Vec<Video>)) {
        self.items.push(element);
    }
}
