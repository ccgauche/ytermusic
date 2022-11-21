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

use crate::{
    consts::CACHE_DIR, structures::sound_action::SoundAction, systems::download, DATABASE,
};

use super::{rect_contains, relative_pos, EventResponse, ManagerMessage, Screen, Screens};

pub struct Chooser {
    pub selected: usize,
    pub items: Vec<PlayListEntry>,
    pub action_sender: Arc<Sender<SoundAction>>,
}

pub struct PlayListEntry {
    pub name: String,
    pub videos: Vec<Video>,
    pub local_videos: usize,
    pub text_to_show: String,
}

impl PlayListEntry {
    pub fn new(name: String, videos: Vec<Video>) -> Self {
        let db = DATABASE.read().unwrap();
        let local_videos = videos
            .iter()
            .filter(|x| db.iter().any(|y| x.video_id == y.video_id))
            .count();
        Self {
            text_to_show: format!(
                "{}     ({}/{} {}%)",
                name,
                local_videos,
                videos.len(),
                (local_videos as f32 / videos.len() as f32 * 100.0) as u8
            ),
            name,
            local_videos,
            videos,
        }
    }

    pub fn tupplelize(&self) -> (&String, &Vec<Video>) {
        (&self.name, &self.videos)
    }
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
        } else if let MouseEventKind::ScrollUp = &mouse_event.kind {
            self.selected(self.selected.saturating_add(1) as isize);
        } else if let MouseEventKind::ScrollDown = &mouse_event.kind {
            self.selected(self.selected.saturating_sub(1) as isize);
        }
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => return ManagerMessage::ChangeState(Screens::MusicPlayer).event(),
            KeyCode::Char('f') => return ManagerMessage::ChangeState(Screens::Search).event(),
            KeyCode::Enter => {
                if let Some(a) = &self.items.get(self.selected) {
                    if a.name != "Local musics" {
                        std::fs::write(
                            CACHE_DIR.join("last-playlist.json"),
                            serde_json::to_string(&a.tupplelize()).unwrap(),
                        )
                        .unwrap();
                    }
                    self.action_sender.send(SoundAction::Cleanup).unwrap();
                    download::clean(self.action_sender.clone());
                    for video in self.items.get(self.selected).unwrap().videos.iter() {
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
                        ListItem::new(i.text_to_show.as_str()).style(
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
        self.items.push(PlayListEntry::new(element.0, element.1));
    }
}
