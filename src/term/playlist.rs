use std::sync::{atomic::AtomicBool, Arc};

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use flume::Sender;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListState},
    Frame,
};
use ytpapi::Video;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    structures::sound_action::SoundAction,
    systems::{download, logger::log_},
    utils::get_before,
    DATABASE,
};

use super::{
    item_list::{ListItem, ListItemAction},
    EventResponse, ManagerMessage, Screen, Screens,
};

#[derive(Clone)]
pub enum ChooserAction {
    Play(PlayListEntry),
}

impl ListItemAction for ChooserAction {
    fn render_style(&self, string: &str, selected: bool) -> Style {
        if selected {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        }
    }
}

pub struct Chooser {
    pub item_list: ListItem<ChooserAction>,
    /* pub selected: usize,

    pub items: Vec<PlayListEntry>, */
    pub action_sender: Arc<Sender<SoundAction>>,
}

#[derive(Clone)]
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
        if let Some(ChooserAction::Play(a)) = self.item_list.on_mouse_press(mouse_event, frame_data)
        {
            if PLAYER_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
                return EventResponse::Message(vec![
                    ManagerMessage::Inspect(a.name, a.videos).pass_to(Screens::PlaylistViewer)
                ]);
            }
            self.play(&a);
            EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::MusicPlayer)])
        } else {
            EventResponse::None
        }
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        if let Some(ChooserAction::Play(a)) = self.item_list.on_key_press(key.clone()).cloned() {
            if PLAYER_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
                return EventResponse::Message(vec![
                    ManagerMessage::Inspect(a.name, a.videos).pass_to(Screens::PlaylistViewer)
                ]);
            }
            self.play(&a);
            return EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::MusicPlayer)]);
        }
        match key.code {
            KeyCode::Esc => return ManagerMessage::ChangeState(Screens::MusicPlayer).event(),
            KeyCode::Char('f') => return ManagerMessage::ChangeState(Screens::Search).event(),
            _ => {}
        }
        EventResponse::None
    }

    fn render(&mut self, frame: &mut Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        frame.render_widget(&self.item_list, frame.size());
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
pub static PLAYER_RUNNING: AtomicBool = AtomicBool::new(false);

impl Chooser {
    fn play(&mut self, a: &PlayListEntry) {
        if a.name != "Local musics" {
            std::fs::write(
                CACHE_DIR.join("last-playlist.json"),
                serde_json::to_string(&a.tupplelize()).unwrap(),
            )
            .unwrap();
        }
        self.action_sender.send(SoundAction::Cleanup).unwrap();
        download::clean(self.action_sender.clone());
        for video in a.videos.iter() {
            download::add(video.clone(), &self.action_sender);
        }
    }
    fn add_element(&mut self, element: (String, Vec<Video>)) {
        let entry = PlayListEntry::new(element.0, element.1);
        self.item_list
            .add_element((entry.text_to_show.clone(), ChooserAction::Play(entry)));
    }
}
