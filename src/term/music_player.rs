use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use player::Player;
use tui::{
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState},
};
use ytpapi::Video;

use crate::{
    systems::player::{generate_music, PlayerState},
    SoundAction,
};

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

pub struct App {
    pub musics: Vec<UIMusic>,
    pub player: PlayerState,
}

impl Screen for App {
    fn on_mouse_press(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
        frame_data: &tui::layout::Rect,
    ) -> EventResponse {
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
                            self.player.apply_sound_action(SoundAction::Next(a));
                        }
                        MusicStatusAction::Current => {
                            self.player.apply_sound_action(SoundAction::PlayPause);
                        }
                        MusicStatusAction::Before(a) => {
                            self.player.apply_sound_action(SoundAction::Previous(a));
                        }
                        MusicStatusAction::Downloading => {}
                    }
                }
            }
        }
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &tui::layout::Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => ManagerMessage::ChangeState(Screens::Playlist).event(),
            KeyCode::Char('f') => ManagerMessage::ChangeState(Screens::Search).event(),
            KeyCode::Char(' ') => {
                self.player.apply_sound_action(SoundAction::PlayPause);
                EventResponse::None
            }
            KeyCode::Char('+') | KeyCode::Up => {
                self.player.apply_sound_action(SoundAction::Plus);
                EventResponse::None
            }
            KeyCode::Char('-') | KeyCode::Down => {
                self.player.apply_sound_action(SoundAction::Minus);
                EventResponse::None
            }
            KeyCode::Char('<') | KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.player.apply_sound_action(SoundAction::Previous(1));
                } else {
                    self.player.apply_sound_action(SoundAction::Backward);
                }
                EventResponse::None
            }
            KeyCode::Char('>') | KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.player.apply_sound_action(SoundAction::Next(1));
                } else {
                    self.player.apply_sound_action(SoundAction::Forward);
                }
                EventResponse::None
            }
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, f: &mut tui::Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        self.musics = generate_music(
            &self.player.queue,
            &self.player.previous,
            &self.player.current,
            &self.player.sink,
        );
        self.player.update();
        let [top_rect, progress_rect] = split_y(f.size(), 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        let colors = if self.player.sink.is_paused() {
            AppStatus::Paused
        } else if self.player.sink.is_finished() {
            AppStatus::NoMusic
        } else {
            AppStatus::Playing
        }
        .colors();
        f.render_widget(
            Gauge::default()
                .block(Block::default().title(" Volume ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio((self.player.sink.volume() as f64).min(1.0).max(0.0)),
            volume_rect,
        );
        let current_time = self.player.sink.elapsed().as_secs();
        let total_time = self.player.sink.duration().map(|x| x as u32).unwrap_or(0);
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
                    if self.player.sink.is_finished() {
                        0.5
                    } else {
                        self.player.sink.percentage().min(100.)
                    }
                    .min(1.0)
                    .max(0.0),
                )
                .label(format!(
                    "{}:{:02} / {}:{:02}",
                    current_time / 60,
                    current_time % 60,
                    total_time / 60,
                    total_time % 60
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

    fn handle_global_message(&mut self, message: ManagerMessage) -> EventResponse {
        match message {
            ManagerMessage::RestartPlayer => {
                self.player.apply_sound_action(SoundAction::RestartPlayer);
                ManagerMessage::ChangeState(Screens::MusicPlayer).event()
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        self.player.apply_sound_action(SoundAction::ForcePause);
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        self.player.apply_sound_action(SoundAction::ForcePlay);
        EventResponse::None
    }
}

impl App {
    pub fn default(player: PlayerState) -> Self {
        Self {
            musics: vec![],
            player,
        }
    }
}
