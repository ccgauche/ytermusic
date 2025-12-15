use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};

use rand::seq::SliceRandom;
use ratatui::widgets::{Block, Borders, Gauge};

use crate::{
    consts::CONFIG,
    errors::handle_error,
    structures::{
        app_status::{AppStatus, MusicDownloadStatus},
        sound_action::SoundAction,
    },
    systems::{download::DOWNLOAD_LIST, player::PlayerState},
    utils::invert,
};

use super::{
    rect_contains, relative_pos, split_x, split_y, vertical_gauge::VerticalGauge, EventResponse,
    ManagerMessage, Screen, Screens,
};

impl PlayerState {
    pub fn activate(&mut self, index: usize) {
        match index.cmp(&self.current) {
            std::cmp::Ordering::Less => {
                SoundAction::Previous(self.current - index).apply_sound_action(self);
            }
            std::cmp::Ordering::Equal => {
                SoundAction::PlayPause.apply_sound_action(self);
            }
            std::cmp::Ordering::Greater => {
                SoundAction::Next(index - self.current).apply_sound_action(self)
            }
        }
    }
}
impl Screen for PlayerState {
    fn on_mouse_press(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
        frame_data: &ratatui::layout::Rect,
    ) -> EventResponse {
        let x = mouse_event.column;
        let y = mouse_event.row;
        let [top_rect, bottom] = split_y(*frame_data, 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        if let MouseEventKind::Down(_) = &mouse_event.kind {
            if rect_contains(&list_rect, x, y, 1) {
                let (_, y) = relative_pos(&list_rect, x, y, 1);
                if let Some(e) = self
                    .list_selector
                    .click_on(y as usize, list_rect.height as usize)
                {
                    self.activate(e);
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

    fn on_key_press(&mut self, key: KeyEvent, _: &ratatui::layout::Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => ManagerMessage::ChangeState(self.goto).event(),
            KeyCode::F(5) => {
                // Get all musics that have failled to download
                let mut musics = Vec::new();
                self.music_status
                    .iter_mut()
                    .for_each(|(key, music_status)| {
                        if MusicDownloadStatus::DownloadFailed != *music_status {
                            return;
                        }
                        if let Some(e) = self.list.iter().find(|x| &x.video_id == key) {
                            musics.push(e.clone());
                            *music_status = MusicDownloadStatus::NotDownloaded;
                        }
                    });
                // Download them
                DOWNLOAD_LIST.lock().unwrap().extend(musics);
                EventResponse::None
            }
            KeyCode::Char('f') => ManagerMessage::SearchFrom(Screens::MusicPlayer).event(),
            KeyCode::Char('s') => {
                self.list.shuffle(&mut rand::thread_rng());
                self.current = 0;
                handle_error(&self.updater, "sink stop", self.sink.stop(&self.guard));
                EventResponse::None
            }
            KeyCode::Char('C') => {
                SoundAction::Cleanup.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char(' ') => {
                SoundAction::PlayPause.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_selector.scroll_up();
                EventResponse::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_selector.scroll_down();
                EventResponse::None
            }
            KeyCode::Enter => {
                if let Some(e) = self.list_selector.play() {
                    self.activate(e);
                }
                EventResponse::None
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                SoundAction::Plus.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char('-') => {
                SoundAction::Minus.apply_sound_action(self);
                EventResponse::None
            }
            KeyCode::Char('<') | KeyCode::Left | KeyCode::Char('h') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    SoundAction::Previous(1).apply_sound_action(self);
                } else {
                    SoundAction::Backward.apply_sound_action(self);
                }
                EventResponse::None
            }
            KeyCode::Char('>') | KeyCode::Right | KeyCode::Char('l') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    SoundAction::Next(1).apply_sound_action(self);
                } else {
                    SoundAction::Forward.apply_sound_action(self);
                }
                EventResponse::None
            }
            KeyCode::Char('r') => {
                SoundAction::DeleteVideoUnary.apply_sound_action(self);
                EventResponse::None
            }
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let render_volume_slider = CONFIG.player.volume_slider;
        let [top_rect, progress_rect] = split_y(f.size(), 3);
        let [list_rect, volume_rect] = split_x(top_rect, if render_volume_slider { 10 } else { 0 });
        let colors = if self.sink.is_paused() {
            AppStatus::Paused
        } else if self.sink.is_finished() {
            AppStatus::NoMusic
        } else {
            AppStatus::Playing
        }
        .style();
        if render_volume_slider {
            f.render_widget(
                VerticalGauge::default()
                    .block(Block::default().title(" Volume ").borders(Borders::ALL))
                    .gauge_style(colors)
                    .ratio((self.sink.volume() as f64 / 100.).clamp(0.0, 1.0)),
                volume_rect,
            );
        }
        let current_time = self.sink.elapsed();
        let total_time = self.sink.duration().map(|x| x as u32).unwrap_or(0);
        f.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .title(
                            self.current()
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
        self.list_selector.update(self.list.len(), self.current);
        self.list_selector.render(
            list_rect,
            f.buffer_mut(),
            |index, select, scroll| {
                let music_state = self
                    .list
                    .get(index)
                    .and_then(|x| self.music_status.get(&x.video_id))
                    .copied()
                    .unwrap_or(MusicDownloadStatus::Downloaded);
                let music_state_c = music_state.character(Some(!self.sink.is_paused()));
                (
                    if select {
                        music_state.style(Some(!self.sink.is_paused()))
                    } else if scroll {
                        invert(music_state.style(None))
                    } else {
                        music_state.style(None)
                    },
                    if let Some(e) = self.list.get(index) {
                        format!(" {music_state_c} {} | {}", e.author, e.title)
                    } else {
                        String::new()
                    },
                )
            },
            " Playlist ",
        )
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
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
