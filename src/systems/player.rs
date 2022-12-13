use std::{collections::VecDeque, sync::Arc};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, PlayError, Player, PlayerOptions, StreamError};

use tui::style::Style;
use ytpapi::Video;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    database,
    errors::{handle_error, handle_error_option},
    structures::{media::Media, music_status::MusicStatus, sound_action::SoundAction},
    term::{
        list_selector::{ListSelector, ListSelectorAction},
        ManagerMessage, Screens,
    },
};

use super::download::IN_DOWNLOAD;

pub enum PlayerAction {
    Current(bool), // Is paused
    Next(usize),
    Previous(usize),
    Downloading,
}

fn invert(style: Style) -> Style {
    Style {
        fg: style.bg,
        bg: style.fg,
        add_modifier: style.add_modifier,
        sub_modifier: style.sub_modifier,
    }
}

impl ListSelectorAction for PlayerAction {
    fn render_style(&self, _: &str, _: bool, scrolling_on: bool) -> Style {
        match self {
            Self::Current(paused) => {
                if *paused {
                    CONFIG.player.text_paused_style
                } else {
                    CONFIG.player.text_playing_style
                }
            }
            Self::Downloading => CONFIG.player.text_downloading_style,
            Self::Next(_) => {
                if scrolling_on {
                    invert(CONFIG.player.text_next_style)
                } else {
                    CONFIG.player.text_next_style
                }
            }
            Self::Previous(_) => {
                if scrolling_on {
                    invert(CONFIG.player.text_previous_style)
                } else {
                    CONFIG.player.text_previous_style
                }
            }
        }
    }
}

pub struct PlayerState {
    pub queue: VecDeque<Video>,
    pub current: Option<Video>,
    pub previous: Vec<Video>,
    pub list_selector: ListSelector<PlayerAction>,
    pub controls: Media,
    pub sink: Player,
    pub guard: Guard,
    pub updater: Arc<Sender<ManagerMessage>>,
    pub soundaction_sender: Arc<Sender<SoundAction>>,
    pub soundaction_receiver: Receiver<SoundAction>,
    pub stream_error_receiver: Receiver<StreamError>,
}

impl PlayerState {
    fn new(
        soundaction_sender: Arc<Sender<SoundAction>>,
        soundaction_receiver: Receiver<SoundAction>,
        updater: Arc<Sender<ManagerMessage>>,
    ) -> Self {
        let (stream_error_sender, stream_error_receiver) = unbounded();
        let (sink, guard) = handle_error_option(
            &updater,
            "player creation error",
            Player::new(
                Arc::new(stream_error_sender),
                PlayerOptions {
                    initial_volume: CONFIG.player.initial_volume,
                },
            ),
        )
        .unwrap();
        Self {
            controls: Media::new(updater.clone(), soundaction_sender.clone()),
            soundaction_receiver,
            list_selector: ListSelector::default(),
            updater,
            stream_error_receiver,
            soundaction_sender,
            sink,
            guard,
            queue: Default::default(),
            current: Default::default(),
            previous: Default::default(),
        }
    }

    pub fn update(&mut self) {
        self.update_controls();
        self.handle_stream_errors();
        while let Ok(e) = self.soundaction_receiver.try_recv() {
            e.apply_sound_action(self);
        }
        if self.sink.is_finished() {
            self.handle_stream_errors();
            self.update_controls();
            if let Some(video) = self.queue.pop_front() {
                let k = CACHE_DIR.join(format!("downloads/{}.mp4", &video.video_id));
                if let Some(e) = self.current.replace(video.clone()) {
                    self.previous.push(e);
                }
                if let Err(e) = self.sink.play(k.as_path(), &self.guard) {
                    if matches!(e, PlayError::DecoderError(_)) {
                        // Cleaning the file

                        database::remove_video(&video);
                        handle_error(
                            &self.updater,
                            "invalid cleaning MP4",
                            std::fs::remove_file(k),
                        );
                        handle_error(
                            &self.updater,
                            "invalid cleaning JSON",
                            std::fs::remove_file(
                                CACHE_DIR.join(format!("downloads/{}.json", &video.video_id)),
                            ),
                        );
                        self.current = None;
                        crate::write();
                    } else {
                        self.updater
                            .send(ManagerMessage::PassTo(
                                Screens::DeviceLost,
                                Box::new(ManagerMessage::Error(format!("{}", e), Box::new(None))),
                            ))
                            .unwrap();
                    }
                }
            } else if let Some(e) = self.current.take() {
                self.previous.push(e);
            }
        }
    }

    fn handle_stream_errors(&self) {
        while let Ok(e) = self.stream_error_receiver.try_recv() {
            handle_error(&self.updater, "audio device stream error", Err(e));
        }
    }
    fn update_controls(&mut self) {
        let result = self
            .controls
            .update(&self.current, &self.sink)
            .map_err(|x| format!("{:?}", x));
        handle_error::<String>(&self.updater, "Can't update finished media control", result);
    }
}

pub fn player_system(
    updater: Arc<Sender<ManagerMessage>>,
) -> (Arc<Sender<SoundAction>>, PlayerState) {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    let tx = Arc::new(tx);
    let k = tx.clone();
    (tx, PlayerState::new(k, rx, updater))
}

pub fn generate_music<'a>(
    queue: &'a VecDeque<Video>,
    previous: &'a [Video],
    current: &'a Option<Video>,
    sink: &'a Player,
) -> Vec<(String, PlayerAction)> {
    let mut music = Vec::with_capacity(10 + queue.len() + previous.len());

    music.extend(IN_DOWNLOAD.lock().unwrap().iter().map(|e| {
        (
            format!(
                " {} [{:02}%] {} | {}",
                MusicStatus::Downloading.character(),
                e.1.clamp(1, 99),
                e.0.author,
                e.0.title,
            ),
            PlayerAction::Downloading,
        )
    }));
    music.extend(previous.iter().rev().enumerate().rev().map(|(i, e)| {
        (
            format!(
                " {} {} | {}",
                MusicStatus::Previous.character(),
                e.author,
                e.title
            ),
            PlayerAction::Previous(i + 1),
        )
    }));
    if let Some(e) = current {
        let status = if sink.is_paused() {
            MusicStatus::Paused.character()
        } else {
            MusicStatus::Playing.character()
        };
        music.push((
            format!(" {status} {} | {}", e.author, e.title),
            PlayerAction::Current(sink.is_paused()),
        ));
    }
    music.extend(queue.iter().enumerate().map(|(i, e)| {
        (
            format!(
                " {} {} | {}",
                MusicStatus::Next.character(),
                e.author,
                e.title
            ),
            PlayerAction::Next(i + 1),
        )
    }));
    music
}
