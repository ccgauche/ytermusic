use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::Ordering, Arc},
};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, PlayError, Player, PlayerOptions, StreamError};

use tui::style::Style;
use ytpapi::Video;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    database,
    errors::{handle_error, handle_error_option},
    structures::{app_status::MusicDownloadStatus, media::Media, sound_action::SoundAction},
    term::{
        list_selector::{ListSelector, ListSelectorAction},
        playlist::PLAYER_RUNNING,
        ManagerMessage, Screens,
    },
    utils::invert,
};

use super::download::DOWNLOAD_LIST;

pub enum PlayerAction {
    Current(MusicDownloadStatus, bool), // Is paused
    Next(MusicDownloadStatus, usize),
    Previous(MusicDownloadStatus, usize),
}

impl ListSelectorAction for PlayerAction {
    fn render_style(&self, _: &str, _: bool, scrolling_on: bool) -> Style {
        match self {
            Self::Current(e, paused) => e.style(Some(!paused)),
            Self::Next(e, _) => {
                if scrolling_on {
                    invert(e.style(None))
                } else {
                    e.style(None)
                }
            }
            Self::Previous(e, _) => {
                if scrolling_on {
                    invert(e.style(None))
                } else {
                    e.style(None)
                }
            }
        }
    }
}

pub struct PlayerState {
    pub goto: Screens,
    pub queue: VecDeque<Video>,
    pub current: Option<Video>,
    pub previous: Vec<Video>,
    pub music_status: HashMap<String, MusicDownloadStatus>,
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
            music_status: HashMap::new(),
            updater,
            stream_error_receiver,
            soundaction_sender,
            sink,
            goto: Screens::Playlist,
            guard,
            queue: Default::default(),
            current: Default::default(),
            previous: Default::default(),
        }
    }

    pub fn update(&mut self) {
        PLAYER_RUNNING.store(self.current.is_some(), Ordering::SeqCst);
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
        let mut to_download = self
            .queue
            .iter()
            .chain(self.previous.iter().rev())
            .filter(|x| {
                self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::NotDownloaded)
            })
            .take(12)
            .cloned()
            .collect::<VecDeque<_>>();
        if let Some(e) = self.current.as_ref() {
            if self.music_status.get(&e.video_id) == Some(&MusicDownloadStatus::NotDownloaded) {
                to_download.push_front(e.clone());
            }
        }
        *DOWNLOAD_LIST.lock().unwrap() = to_download;
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
    music_status: &'a HashMap<String, MusicDownloadStatus>,
    previous: &'a [Video],
    current: &'a Option<Video>,
    sink: &'a Player,
) -> Vec<(String, PlayerAction)> {
    let mut music = Vec::with_capacity(10 + queue.len() + previous.len());

    music.extend(previous.iter().rev().enumerate().rev().map(|(i, e)| {
        let status = music_status
            .get(&e.video_id)
            .copied()
            .unwrap_or(MusicDownloadStatus::Downloaded);
        (
            format!(" {} {} | {}", status.character(None), e.author, e.title),
            PlayerAction::Previous(status, i + 1),
        )
    }));
    if let Some(e) = current {
        let mstatus = music_status
            .get(&e.video_id)
            .copied()
            .unwrap_or(MusicDownloadStatus::Downloaded);
        let status = mstatus.character(Some(!sink.is_paused()));

        music.push((
            format!(" {status} {} | {}", e.author, e.title),
            PlayerAction::Current(mstatus, sink.is_paused()),
        ));
    }
    music.extend(queue.iter().enumerate().map(|(i, e)| {
        let status = music_status
            .get(&e.video_id)
            .copied()
            .unwrap_or(MusicDownloadStatus::Downloaded);
        (
            format!(" {} {} | {}", status.character(None), e.author, e.title),
            PlayerAction::Next(status, i + 1),
        )
    }));
    music
}
