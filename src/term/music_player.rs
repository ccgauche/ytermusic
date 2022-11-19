use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use tui::{
    style::Style,
    widgets::{Block, Borders, Gauge, List, ListState},
};

use crate::{
    structures::{
        app_status::AppStatus, music_status_action::MusicStatusAction, sound_action::SoundAction,
    },
    systems::player::{generate_music, get_action, PlayerState},
};

use super::{
    rect_contains, relative_pos, split_x, split_y, EventResponse, ManagerMessage, Screen, Screens,
};

impl Screen for PlayerState {
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
                match get_action(y as usize, &self.queue, &self.previous, &self.current) {
                    Some(MusicStatusAction::Skip(a)) => {
                        SoundAction::Next(a).apply_sound_action(self);
                    }
                    Some(MusicStatusAction::Current) => {
                        SoundAction::PlayPause.apply_sound_action(self);
                    }
                    Some(MusicStatusAction::Before(a)) => {
                        SoundAction::Previous(a).apply_sound_action(self);
                    }
                    None | Some(MusicStatusAction::Downloading) => (),
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
                SoundAction::PlayPause.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char('+') | KeyCode::Up => {
                SoundAction::Plus.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char('-') | KeyCode::Down => {
                SoundAction::Minus.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char('<') | KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    SoundAction::Previous(1).apply_sound_action(self);
                } else {
                    SoundAction::Backward.apply_sound_action(self);
                }
                EventResponse::None
            }
            KeyCode::Char('>') | KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    SoundAction::Next(1).apply_sound_action(self);
                } else {
                    SoundAction::Forward.apply_sound_action(self);
                }
                EventResponse::None
            }
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, f: &mut tui::Frame<tui::backend::CrosstermBackend<std::io::Stdout>>) {
        self.update();
        let [top_rect, progress_rect] = split_y(f.size(), 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        let colors = if self.sink.is_paused() {
            AppStatus::Paused
        } else if self.sink.is_finished() {
            AppStatus::NoMusic
        } else {
            AppStatus::Playing
        }
        .colors();
        f.render_widget(
            Gauge::default()
                .block(Block::default().title(" Volume ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio((self.sink.volume() as f64 / 100.).clamp(0.0, 1.0)),
            volume_rect,
        );
        let current_time = self.sink.elapsed().as_secs();
        let total_time = self.sink.duration().map(|x| x as u32).unwrap_or(0);
        f.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .title(
                            self.current
                                .as_ref()
                                .map(|x| format!(" {} | {} ", x.author, x.title))
                                .unwrap_or_else(|| " No music playing ".to_owned()),
                        )
                        .borders(Borders::ALL),
                )
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio(
                    if self.sink.is_finished() {
                        0.5
                    } else {
                        self.sink.percentage().min(100.)
                    }
                    .clamp(0.0, 1.0),
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
            List::new(generate_music(
                f.size().height as usize,
                &self.queue,
                &self.previous,
                &self.current,
                &self.sink,
            ))
            .block(Block::default().borders(Borders::ALL).title(" Playlist ")),
            list_rect,
            &mut ListState::default(),
        );
    }

    fn handle_global_message(&mut self, message: ManagerMessage) -> EventResponse {
        match message {
            ManagerMessage::RestartPlayer => {
                SoundAction::RestartPlayer.apply_sound_action(self);
                ManagerMessage::ChangeState(Screens::MusicPlayer).event()
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        //SoundAction::ForcePause.apply_sound_action(self);
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        //SoundAction::ForcePlay.apply_sound_action(self);
        EventResponse::None
    }
}
