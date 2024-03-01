use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::Ordering,
};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, PlayError, Player, PlayerOptions, StreamError};

use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    database,
    errors::{handle_error, handle_error_option},
    structures::{app_status::MusicDownloadStatus, media::Media, sound_action::SoundAction},
    term::{
        list_selector::ListSelector, playlist::PLAYER_RUNNING,
        ManagerMessage, Screens,
    },
};

use super::download::DOWNLOAD_LIST;

pub struct PlayerState {
    pub goto: Screens,
    pub list: Vec<YoutubeMusicVideoRef>,
    pub current: usize,
    pub rtcurrent: Option<YoutubeMusicVideoRef>,
    pub music_status: HashMap<String, MusicDownloadStatus>,
    pub list_selector: ListSelector,
    pub controls: Media,
    pub sink: Player,
    pub guard: Guard,
    pub updater: Sender<ManagerMessage>,
    pub soundaction_sender: Sender<SoundAction>,
    pub soundaction_receiver: Receiver<SoundAction>,
    pub stream_error_receiver: Receiver<StreamError>,
}

impl PlayerState {
    fn new(
        soundaction_sender: Sender<SoundAction>,
        soundaction_receiver: Receiver<SoundAction>,
        updater: Sender<ManagerMessage>,
    ) -> Self {
        let (stream_error_sender, stream_error_receiver) = unbounded::<StreamError>();
        let (sink, guard) = handle_error_option(
            &updater,
            "player creation error",
            Player::new(
                stream_error_sender,
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
            list: Vec::new(),
            current: 0,
            rtcurrent: None,
        }
    }

    pub fn current(&self) -> Option<&YoutubeMusicVideoRef> {
        self.relative_current(0)
    }

    pub fn relative_current(&self, n: isize) -> Option<&YoutubeMusicVideoRef> {
        self.list.get(self.current.saturating_add_signed(n))
    }

    pub fn set_relative_current(&mut self, n: isize) {
        self.current = self.current.saturating_add_signed(n);
    }

    pub fn update(&mut self) {
        PLAYER_RUNNING.store(self.current().is_some(), Ordering::SeqCst);
        self.update_controls();
        self.handle_stream_errors();
        while let Ok(e) = self.soundaction_receiver.try_recv() {
            e.apply_sound_action(self);
        }
        if self
            .current()
            .as_ref()
            .map(|x| {
                self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::DownloadFailed)
            })
            .unwrap_or(false)
        {
            SoundAction::Next(1).apply_sound_action(self);
        }
        if self.sink.is_finished() {
            if self
                .rtcurrent
                .as_ref()
                .zip(self.current())
                .map(|(x, y)| {
                    x == y
                        && self.music_status.get(&x.video_id)
                            == Some(&MusicDownloadStatus::Downloaded)
                })
                .unwrap_or(false)
            {
                self.set_relative_current(1);
            }
            self.handle_stream_errors();
            self.update_controls();
            // If the current song is finished, we play the next one but if the next one has failed to download, we skip it
            // TODO(optimize this)
            while self
                .current()
                .map(|x| {
                    self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::DownloadFailed)
                })
                .unwrap_or(false)
            {
                self.set_relative_current(1);
            }

            if !self
                .current()
                .map(|x| {
                    self.music_status.get(&x.video_id) != Some(&MusicDownloadStatus::Downloaded)
                })
                .unwrap_or(true)
            {
                if let Some(video) = self.current().cloned() {
                    let k = CACHE_DIR.join(format!("downloads/{}.mp4", &video.video_id));
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
                            self.current = 0;
                            crate::write();
                        } else {
                            self.updater
                                .send(ManagerMessage::PassTo(
                                    Screens::DeviceLost,
                                    Box::new(ManagerMessage::Error(format!("{e}"), Box::new(None))),
                                ))
                                .unwrap();
                        }
                    }
                }
            }
        }
        self.rtcurrent = self.current().cloned();
        let to_download = self
            .list
            .iter()
            .skip(self.current)
            .chain(self.list.iter().take(self.current).rev())
            .filter(|x| {
                self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::NotDownloaded)
            })
            .take(12)
            .cloned()
            .collect::<VecDeque<_>>();
        *DOWNLOAD_LIST.lock().unwrap() = to_download;
    }

    fn handle_stream_errors(&self) {
        while let Ok(e) = self.stream_error_receiver.try_recv() {
            handle_error(&self.updater, "audio device stream error", Err(e));
        }
    }
    fn update_controls(&mut self) {
        let current = self.current().cloned();
        let result = self
            .controls
            .update(current, &self.sink)
            .map_err(|x| format!("{x:?}"));
        handle_error::<String>(&self.updater, "Can't update finished media control", result);
    }
}

pub fn player_system(updater: Sender<ManagerMessage>) -> (Sender<SoundAction>, PlayerState) {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    (tx.clone(), PlayerState::new(tx, rx, updater))
}
