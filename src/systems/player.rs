use std::{collections::VecDeque, sync::Arc};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, PlayError, Player, PlayerOptions, StreamError};

use tui::{style::Style, widgets::ListItem};
use ytpapi::Video;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    database,
    errors::{handle_error, handle_error_option},
    structures::{
        media::Media, music_status::MusicStatus, music_status_action::MusicStatusAction,
        sound_action::SoundAction,
    },
    term::{ManagerMessage, Screens},
    utils::generate_music_repartition,
};

use super::download::IN_DOWNLOAD;

pub struct PlayerState {
    pub queue: VecDeque<Video>,
    pub current: Option<Video>,
    pub previous: Vec<Video>,
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
                                Box::new(ManagerMessage::Error(format!("{:?}", e))),
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

// https://docs.rs/souvlaki/latest/souvlaki/

pub fn get_action(
    mut index: usize,
    lines: usize,
    queue: &VecDeque<Video>,
    previous: &[Video],
    current: &Option<Video>,
) -> Option<MusicStatusAction> {
    let dw_len = IN_DOWNLOAD.lock().unwrap().len();
    if index < dw_len {
        return Some(MusicStatusAction::Downloading);
    }
    index -= dw_len;
    let (previous_max, _) = generate_music_repartition(lines, queue, previous, current);
    let previous_len = previous.len().min(previous_max);
    if index < previous_len {
        return Some(MusicStatusAction::Before(previous_len - index));
    }
    index -= previous_len;
    if current.is_some() {
        if index == 0 {
            return Some(MusicStatusAction::Current);
        }
        index -= 1;
    }
    if queue.len() < index {
        None
    } else {
        Some(MusicStatusAction::Skip(index + 1))
    }
}

pub fn generate_music<'a>(
    lines: usize,
    queue: &'a VecDeque<Video>,
    previous: &'a [Video],
    current: &'a Option<Video>,
    sink: &'a Player,
) -> Vec<ListItem<'a>> {
    let download_style: Style = CONFIG.player.text_downloading_style;
    let previous_style: Style = CONFIG.player.text_previous_style;
    let paused_style: Style = CONFIG.player.text_paused_style;
    let playing_style: Style = CONFIG.player.text_playing_style;
    let next_style: Style = CONFIG.player.text_next_style;

    let mut music = Vec::with_capacity(50);
    let (before, after) = generate_music_repartition(lines, queue, previous, current);
    {
        music.extend(IN_DOWNLOAD.lock().unwrap().iter().map(|e| {
            ListItem::new(format!(
                " {} [{:02}%] {} | {}",
                MusicStatus::Downloading.character(),
                e.1.clamp(1, 99),
                e.0.author,
                e.0.title,
            ))
            .style(download_style)
        }));
        music.extend(previous.iter().rev().take(before).rev().map(|e| {
            ListItem::new(format!(
                " {} {} | {}",
                MusicStatus::Previous.character(),
                e.author,
                e.title
            ))
            .style(previous_style)
        }));
        if let Some(e) = current {
            let status = if sink.is_paused() {
                (MusicStatus::Paused.character(), paused_style)
            } else {
                (MusicStatus::Playing.character(), playing_style)
            };
            music.push(
                ListItem::new(format!(" {} {} | {}", status.0, e.author, e.title)).style(status.1),
            );
        }
        music.extend(queue.iter().take(after + 4).map(|e| {
            ListItem::new(format!(
                " {} {} | {}",
                MusicStatus::Next.character(),
                e.author,
                e.title
            ))
            .style(next_style)
        }));
    }
    music
}
