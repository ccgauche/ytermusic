use std::sync::{atomic::AtomicBool, Arc};

use crossterm::event::{KeyCode, KeyEvent};
use flume::Sender;
use tui::{
    layout::Rect,
    style::{Color, Style},
    Frame,
};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    consts::CACHE_DIR, structures::sound_action::SoundAction, systems::download, DATABASE,
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
    fn render_style(&self, _: &str, selected: bool) -> Style {
        if selected {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        }
    }
}

pub struct Chooser {
    pub item_list: ListItem<ChooserAction>,
    pub goto: Screens,
    pub action_sender: Arc<Sender<SoundAction>>,
}

#[derive(Clone)]
pub struct PlayListEntry {
    pub name: String,
    pub videos: Vec<YoutubeMusicVideoRef>,
    pub text_to_show: String,
}

impl PlayListEntry {
    pub fn new(name: String, videos: Vec<YoutubeMusicVideoRef>) -> Self {
        Self {
            text_to_show: format_playlist(&name, &videos),
            name,
            videos,
        }
    }

    pub fn tupplelize(&self) -> (&String, &Vec<YoutubeMusicVideoRef>) {
        (&self.name, &self.videos)
    }
}
pub fn format_playlist(name: &str, videos: &[YoutubeMusicVideoRef]) -> String {
    let db = DATABASE.read().unwrap();
    let local_videos = videos
        .iter()
        .filter(|x| db.iter().any(|y| x.video_id == y.video_id))
        .count();
    format!(
        "{}     ({}/{} {}%)",
        name,
        local_videos,
        videos.len(),
        (local_videos as f32 / videos.len() as f32 * 100.0) as u8
    )
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
                return EventResponse::Message(vec![ManagerMessage::Inspect(
                    a.name,
                    Screens::Playlist,
                    a.videos,
                )
                .pass_to(Screens::PlaylistViewer)]);
            }
            self.play(&a);
            EventResponse::Message(vec![ManagerMessage::PlayerFrom(Screens::Playlist)])
        } else {
            EventResponse::None
        }
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        if let Some(ChooserAction::Play(a)) = self.item_list.on_key_press(key).cloned() {
            if PLAYER_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
                return EventResponse::Message(vec![ManagerMessage::Inspect(
                    a.name,
                    Screens::Playlist,
                    a.videos,
                )
                .pass_to(Screens::PlaylistViewer)]);
            }
            self.play(&a);
            return EventResponse::Message(vec![ManagerMessage::ChangeState(Screens::MusicPlayer)]);
        }
        match key.code {
            KeyCode::Esc => return ManagerMessage::ChangeState(Screens::MusicPlayer).event(),
            KeyCode::Char('f') => return ManagerMessage::SearchFrom(Screens::Playlist).event(),
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
        self.action_sender
            .send(SoundAction::AddVideosToQueue(a.videos.clone()))
            .unwrap();
    }
    fn add_element(&mut self, element: (String, Vec<YoutubeMusicVideoRef>)) {
        let entry = PlayListEntry::new(element.0, element.1);
        self.item_list
            .add_element((entry.text_to_show.clone(), ChooserAction::Play(entry)));
    }
}
