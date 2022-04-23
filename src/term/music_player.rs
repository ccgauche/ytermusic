use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use flume::Sender;
use player::Player;
use tui::{
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState},
};
use ytpapi::Video;

use crate::{systems::download, SoundAction};

use super::{
    rect_contains, relative_pos, split_x, split_y, EventResponse, ManagerMessage, Screen, Screens,
};

#[derive(Debug, Clone)]
pub struct UIMusic {
    pub status: MusicStatus,
    pub title: String,
    pub author: String,
    pub position: MusicStatusAction,
}

#[derive(Debug, Clone)]
pub enum MusicStatusAction {
    Skip(usize),
    Current,
    Before(usize),
    Downloading,
}

impl UIMusic {
    pub fn new(video: &Video, status: MusicStatus, position: MusicStatusAction) -> UIMusic {
        UIMusic {
            status,
            title: video.title.clone(),
            author: video.author.clone(),
            position,
        }
    }
}

impl UIMusic {
    fn text(&self) -> String {
        format!(
            " {} {} | {}",
            self.status.character(),
            self.author,
            self.title
        )
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum AppStatus {
    Paused,
    Playing,
    NoMusic,
}

impl AppStatus {
    pub fn new(sink: &Player) -> Self {
        if sink.is_finished() {
            Self::NoMusic
        } else if sink.is_paused() {
            Self::Paused
        } else {
            Self::Playing
        }
    }
}

impl AppStatus {
    fn colors(&self) -> (Color, Color) {
        match self {
            AppStatus::Paused => (Color::Yellow, Color::Black),
            AppStatus::Playing => (Color::Green, Color::Black),
            AppStatus::NoMusic => (Color::White, Color::Black),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum MusicStatus {
    Playing,
    Paused,
    Previous,
    Next,
    Downloading,
}

impl MusicStatus {
    fn character(&self) -> char {
        match self {
            MusicStatus::Playing => '▶',
            MusicStatus::Paused => '⋈',
            MusicStatus::Previous => ' ',
            MusicStatus::Next => ' ',
            MusicStatus::Downloading => '⮁',
        }
    }

    fn colors(&self) -> (Color, Color) {
        match self {
            MusicStatus::Playing => (Color::Green, Color::Black),
            MusicStatus::Paused => (Color::Yellow, Color::Black),
            MusicStatus::Previous => (Color::White, Color::Black),
            MusicStatus::Next => (Color::White, Color::Black),
            MusicStatus::Downloading => (Color::Blue, Color::Black),
        }
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub musics: Vec<UIMusic>,
    pub app_status: AppStatus,
    pub current_time: u32,
    pub total_time: u32,
    pub volume: f32,
    pub action_sender: Arc<Sender<SoundAction>>,
}

impl Screen for App {
    fn on_mouse_press(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
        frame_data: &tui::layout::Rect,
    ) -> super::EventResponse {
        if let MouseEventKind::Down(_) = &mouse_event.kind {
            let x = mouse_event.column;
            let y = mouse_event.row;
            let [top_rect, _] = split_y(*frame_data, 3);
            let [list_rect, _] = split_x(top_rect, 10);
            if rect_contains(&list_rect, x, y, 1) {
                let (_, y) = relative_pos(&list_rect, x, y, 1);
                if self.musics.len() > y as usize {
                    let o = &self.musics[y as usize];
                    match o.position {
                        MusicStatusAction::Skip(a) => {
                            self.action_sender.send(SoundAction::Next(a)).unwrap();
                        }
                        MusicStatusAction::Current => {
                            self.action_sender.send(SoundAction::PlayPause).unwrap();
                        }
                        MusicStatusAction::Before(a) => {
                            self.action_sender.send(SoundAction::Previous(a)).unwrap();
                        }
                        MusicStatusAction::Downloading => {}
                    }
                }
            }
        }
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &tui::layout::Rect) -> super::EventResponse {
        if KeyCode::Esc == key.code {
            return super::EventResponse::Message(vec![ManagerMessage::ChangeState(
                Screens::Playlist,
            )]);
        }
        if KeyCode::Char('f') == key.code {
            return super::EventResponse::Message(vec![ManagerMessage::ChangeState(
                Screens::Search,
            )]);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('<') | KeyCode::Left => {
                    self.action_sender.send(SoundAction::Previous(1)).unwrap();
                }
                KeyCode::Char('>') | KeyCode::Right => {
                    self.action_sender.send(SoundAction::Next(1)).unwrap();
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char(' ') => {
                    self.action_sender.send(SoundAction::PlayPause).unwrap();
                }
                KeyCode::Char('<') | KeyCode::Left => {
                    self.action_sender.send(SoundAction::Backward).unwrap();
                }
                KeyCode::Char('>') | KeyCode::Right => {
                    self.action_sender.send(SoundAction::Forward).unwrap();
                }
                KeyCode::Char('+') | KeyCode::Up => {
                    self.action_sender.send(SoundAction::Plus).unwrap();
                }
                KeyCode::Char('-') | KeyCode::Down => {
                    self.action_sender.send(SoundAction::Minus).unwrap();
                }
                _ => {}
            }
        }
        EventResponse::None
    }

    fn render(&mut self, f: &mut tui::Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        let [top_rect, progress_rect] = split_y(f.size(), 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        let colors = self.app_status.colors();
        f.render_widget(
            Gauge::default()
                .block(Block::default().title(" Volume ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio((self.volume as f64).min(1.0).max(0.0)),
            volume_rect,
        );
        f.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .title(
                            self.musics
                                .iter()
                                .find(|x| {
                                    x.status == MusicStatus::Playing
                                        || x.status == MusicStatus::Paused
                                })
                                .map(|x| format!(" {} | {} ", x.author, x.title))
                                .unwrap_or_else(|| " No music playing ".to_owned()),
                        )
                        .borders(Borders::ALL),
                )
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio(
                    if self.app_status == AppStatus::NoMusic {
                        0.5
                    } else {
                        self.current_time as f64 / self.total_time as f64
                    }
                    .min(1.0)
                    .max(0.0),
                )
                .label(format!(
                    "{}:{:02} / {}:{:02}",
                    self.current_time / 60,
                    self.current_time % 60,
                    self.total_time / 60,
                    self.total_time % 60
                )),
            progress_rect,
        );
        // Create a List from all list items and highlight the currently selected one
        f.render_stateful_widget(
            List::new(
                self.musics
                    .iter()
                    .map(|i| {
                        ListItem::new(i.text()).style(
                            Style::default()
                                .fg(i.status.colors().0)
                                .bg(i.status.colors().1),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title(" Playlist ")),
            list_rect,
            &mut ListState::default(),
        );
    }

    fn handle_global_message(&mut self, message: super::ManagerMessage) -> super::EventResponse {
        match message {
            ManagerMessage::UpdateApp(app) => {
                *self = app;
                super::EventResponse::None
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> super::EventResponse {
        self.action_sender.send(SoundAction::Cleanup).unwrap();
        download::clean(self.action_sender.clone());
        EventResponse::None
    }

    fn open(&mut self) -> super::EventResponse {
        EventResponse::None
    }
}

impl App {
    pub fn default(action_sender: Arc<Sender<SoundAction>>) -> Self {
        Self {
            musics: vec![],
            app_status: AppStatus::NoMusic,
            current_time: 0,
            total_time: 0,
            volume: 0.5,
            action_sender,
        }
    }
    pub fn new(
        sink: &Player,
        musics: Vec<UIMusic>,
        action_sender: Arc<Sender<SoundAction>>,
    ) -> Self {
        Self {
            musics,
            app_status: AppStatus::new(sink),
            current_time: sink.elapsed().as_secs() as u32,
            total_time: sink.duration().map(|x| x as u32).unwrap_or(1),
            volume: sink.volume_percent() as f32 / 100.,
            action_sender,
        }
    }
}
