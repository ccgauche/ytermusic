use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use rand::seq::SliceRandom;
use tui::widgets::{Block, Borders, Gauge};

use crate::{
    errors::handle_error,
    structures::{app_status::AppStatus, sound_action::SoundAction},
    systems::player::{generate_music, PlayerAction, PlayerState},
};

use super::{
    rect_contains, relative_pos, split_x, split_y, vertical_gauge::VerticalGauge, EventResponse,
    ManagerMessage, Screen, Screens,
};

impl Screen for PlayerState {
    fn on_mouse_press(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
        frame_data: &tui::layout::Rect,
    ) -> EventResponse {
        let x = mouse_event.column;
        let y = mouse_event.row;
        let [top_rect, bottom] = split_y(*frame_data, 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        if let MouseEventKind::Down(_) = &mouse_event.kind {
            if rect_contains(&list_rect, x, y, 1) {
                let (_, y) = relative_pos(&list_rect, x, y, 1);
                match self
                    .list_selector
                    .click_on(y as usize, list_rect.height as usize)
                {
                    None => {}
                    Some((_, PlayerAction::Current(..))) => {
                        SoundAction::PlayPause.apply_sound_action(self);
                    }
                    Some((_, PlayerAction::Next(_, a))) => {
                        SoundAction::Next(*a).apply_sound_action(self);
                    }
                    Some((_, PlayerAction::Previous(_, a))) => {
                        SoundAction::Previous(*a).apply_sound_action(self);
                    }
                }
            }
            if rect_contains(&bottom, x, y, 1) {
                let (x, _) = relative_pos(&bottom, x, y, 1);
                let size = bottom.width as usize - 2;
                let percent = x as f64 / size as f64;
                if let Some(duration) = self.sink.duration() {
                    let new_position = (duration * 1000. * percent) as u64;
                    self.sink
                        .seek_to(std::time::Duration::from_millis(new_position));
                }
            }
            if rect_contains(&volume_rect, x, y, 1) {
                let (_, y) = relative_pos(&volume_rect, x, y, 1);
                let size = volume_rect.height as usize - 2;
                let percent = 100. - y as f64 / size as f64 * 100.;
                self.sink.set_volume(percent as i32)
            }
        } else if let MouseEventKind::ScrollUp = &mouse_event.kind {
            if rect_contains(&volume_rect, x, y, 1) {
                SoundAction::Plus.apply_sound_action(self);
            } else if rect_contains(&bottom, x, y, 1) {
                SoundAction::Forward.apply_sound_action(self);
            } else {
                self.list_selector.scroll_up();
            }
        } else if let MouseEventKind::ScrollDown = &mouse_event.kind {
            if rect_contains(&volume_rect, x, y, 1) {
                SoundAction::Minus.apply_sound_action(self);
            } else if rect_contains(&bottom, x, y, 1) {
                SoundAction::Backward.apply_sound_action(self);
            } else {
                self.list_selector.scroll_down();
            }
        }
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &tui::layout::Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => ManagerMessage::ChangeState(self.goto).event(),
            KeyCode::Char('f') => ManagerMessage::SearchFrom(Screens::MusicPlayer).event(),
            KeyCode::Char('s') => {
                let mut musics = Vec::with_capacity(self.previous.len() + self.queue.len() + 1);
                musics.append(&mut self.previous);
                if let Some(e) = self.current.take() {
                    musics.push(e);
                }
                let queue = std::mem::take(&mut self.queue);
                musics.extend(queue.into_iter());
                musics.shuffle(&mut rand::thread_rng());
                self.queue = musics.into();
                handle_error(&self.updater, "sink stop", self.sink.stop(&self.guard));
                EventResponse::None
            }
            KeyCode::Char(' ') => {
                SoundAction::PlayPause.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char('+') => {
                SoundAction::Plus.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Up => {
                self.list_selector.scroll_up();
                EventResponse::None
            }
            KeyCode::Down => {
                self.list_selector.scroll_down();
                EventResponse::None
            }
            KeyCode::Enter => {
                if let Some(e) = self.list_selector.play() {
                    match e {
                        PlayerAction::Current(..) => {
                            SoundAction::PlayPause.apply_sound_action(self);
                        }
                        PlayerAction::Next(_, a) => {
                            SoundAction::Next(*a).apply_sound_action(self);
                        }
                        PlayerAction::Previous(_, a) => {
                            SoundAction::Previous(*a).apply_sound_action(self);
                        }
                    }
                }
                EventResponse::None
            }
            KeyCode::Char('-') => {
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
        let [top_rect, progress_rect] = split_y(f.size(), 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        let colors = if self.sink.is_paused() {
            AppStatus::Paused
        } else if self.sink.is_finished() {
            AppStatus::NoMusic
        } else {
            AppStatus::Playing
        }
        .style();
        f.render_widget(
            VerticalGauge::default()
                .block(Block::default().title(" Volume ").borders(Borders::ALL))
                .gauge_style(colors)
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
                                .map(|x| format!(" {x} "))
                                .unwrap_or_else(|| " No music playing ".to_owned()),
                        )
                        .borders(Borders::ALL),
                )
                .gauge_style(colors)
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
        self.list_selector.update(
            generate_music(
                &self.queue,
                &self.music_status,
                &self.previous,
                &self.current,
                &self.sink,
            ),
            self.previous.len(),
        );
        f.render_widget(&self.list_selector, list_rect);
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
