use std::{collections::VecDeque, path::PathBuf, process::exit, str::FromStr, sync::Arc};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, PlayError, Player, StreamError};
use souvlaki::{Error, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};

use tui::{style::Style, widgets::ListItem};
use ytpapi::Video;

use crate::{
    errors::{handle_error, handle_error_option},
    term::{
        music_player::{MusicStatus, MusicStatusAction},
        ManagerMessage, Screens,
    },
    SoundAction, DATABASE,
};

use super::download::{DOWNLOAD_MORE, IN_DOWNLOAD};

#[cfg(not(target_os = "windows"))]
fn get_handle(updater: &Sender<ManagerMessage>) -> Option<MediaControls> {
    handle_error_option(
        updater,
        "Can't create media controls",
        MediaControls::new(PlatformConfig {
            dbus_name: "ytermusic",
            display_name: "YTerMusic",
            hwnd: None,
        }),
    )
}

#[cfg(target_os = "windows")]
fn get_handle(updater: &Sender<ManagerMessage>) -> Option<MediaControls> {
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
    use winit::event_loop::EventLoop;
    use winit::{platform::windows::EventLoopExtWindows, window::WindowBuilder};

    let config = PlatformConfig {
        dbus_name: "ytermusic",
        display_name: "YTerMusic",
        hwnd: if let RawWindowHandle::Win32(h) = handle_error_option(
            updater,
            "OS Error while creating media hook window",
            WindowBuilder::new()
                .with_visible(false)
                .build(&EventLoop::<()>::new_any_thread()),
        )?
        .raw_window_handle()
        {
            Some(h.hwnd)
        } else {
            updater
                .send(ManagerMessage::PassTo(
                    Screens::DeviceLost,
                    Box::new(ManagerMessage::Error(format!("No window handle found"))),
                ))
                .unwrap();
            return None;
        },
    };

    handle_error_option(
        updater,
        "Can't create media controls",
        MediaControls::new(config).map_err(|x| format!("{:?}", x)),
    )
}

pub struct PlayerState {
    pub queue: VecDeque<Video>,
    pub current: Option<Video>,
    pub previous: Vec<Video>,
    pub controls: Option<MediaControls>,
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
            Player::new(Arc::new(stream_error_sender)),
        )
        .unwrap();
        let mut controls = get_handle(&updater);
        if let Some(e) = &mut controls {
            handle_error(
                &updater,
                "Can't connect media control",
                connect(e, soundaction_sender.clone()).map_err(|x| format!("{:?}", x)),
            );
        }
        Self {
            soundaction_receiver,
            updater,
            stream_error_receiver,
            soundaction_sender,
            controls,
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
            self.apply_sound_action(e);
        }
        if self.sink.is_finished() {
            self.handle_stream_errors();
            self.update_controls();
            if let Some(video) = self.queue.pop_front() {
                let k =
                    PathBuf::from_str(&format!("data/downloads/{}.mp4", &video.video_id)).unwrap();
                if let Some(e) = self.current.replace(video.clone()) {
                    self.previous.push(e);
                }
                if let Err(e) = self.sink.play(k.as_path(), &self.guard) {
                    if matches!(e, PlayError::DecoderError(_)) {
                        // Cleaning the file
                        DATABASE
                            .write()
                            .unwrap()
                            .retain(|x| x.video_id != video.video_id);
                        handle_error(
                            &self.updater,
                            "invalid cleaning MP4",
                            std::fs::remove_file(k),
                        );
                        handle_error(
                            &self.updater,
                            "invalid cleaning JSON",
                            std::fs::remove_file(format!(
                                "data/downloads/{}.json",
                                &video.video_id
                            )),
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
            } else {
                if let Some(e) = self.current.take() {
                    self.previous.push(e);
                }
            }
        }
    }

    fn handle_stream_errors(&self) {
        while let Ok(e) = self.stream_error_receiver.try_recv() {
            let _ = handle_error(&self.updater, "audio device stream error", Err(e));
        }
    }
    fn update_controls(&mut self) {
        handle_error::<String>(&self.updater, "Can't update finished media control", {
            let k: Result<_, Error> = try {
                if let Some(e) = &mut self.controls {
                    e.set_metadata(MediaMetadata {
                        title: self.current.as_ref().map(|video| video.title.as_str()),
                        album: self.current.as_ref().map(|video| video.album.as_str()),
                        artist: self.current.as_ref().map(|video| video.author.as_str()),
                        cover_url: None,
                        duration: None,
                    })?;
                    if self.sink.is_finished() {
                        e.set_playback(MediaPlayback::Stopped)?;
                    } else if self.sink.is_paused() {
                        e.set_playback(MediaPlayback::Paused {
                            progress: Some(MediaPosition(self.sink.elapsed())),
                        })?;
                    } else {
                        e.set_playback(MediaPlayback::Playing {
                            progress: Some(MediaPosition(self.sink.elapsed())),
                        })?;
                    }
                }
                ()
            };
            k.map_err(|x| format!("{:?}", x))
        });
    }
    pub fn apply_sound_action(&mut self, e: SoundAction) {
        match e {
            SoundAction::Backward => self.sink.seek_bw(),
            SoundAction::Forward => self.sink.seek_fw(),
            SoundAction::PlayPause => self.sink.toggle_playback(),
            SoundAction::Cleanup => {
                self.queue.clear();
                self.previous.clear();
                self.current = None;
                handle_error(&self.updater, "sink stop", self.sink.stop(&self.guard));
            }
            SoundAction::Plus => self.sink.volume_up(),
            SoundAction::Minus => self.sink.volume_down(),
            SoundAction::Next(a) => {
                for _ in 1..a {
                    if let Some(e) = self.queue.pop_front() {
                        self.previous.push(e);
                    }
                }

                handle_error(&self.updater, "sink stop", self.sink.stop(&self.guard));
            }
            SoundAction::PlayVideo(video) => {
                self.queue.push_back(video);
            }
            SoundAction::Previous(a) => {
                for _ in 0..a {
                    if let Some(e) = self.previous.pop() {
                        if let Some(c) = self.current.take() {
                            self.queue.push_front(c);
                        }
                        self.queue.push_front(e);
                    }
                }
                handle_error(&self.updater, "sink stop", self.sink.stop(&self.guard));
            }
            SoundAction::RestartPlayer => {
                (self.sink, self.guard) =
                    handle_error_option(&self.updater, "update player", self.sink.update())
                        .unwrap();
                if let Some(e) = self.current.clone() {
                    self.apply_sound_action(SoundAction::PlayVideo(e));
                }
            }
            SoundAction::ForcePause => {
                if !self.sink.is_paused() && !self.sink.is_finished() {
                    self.sink.pause();
                }
            }
            SoundAction::ForcePlay => {
                if self.sink.is_paused() && !self.sink.is_finished() {
                    self.sink.pause();
                }
            }
            SoundAction::PlayVideoUnary(video) => {
                self.queue.push_front(video);
            }
        }
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

fn connect(mpris: &mut MediaControls, sender: Arc<Sender<SoundAction>>) -> Result<(), Error> {
    mpris.attach(move |e| match e {
        souvlaki::MediaControlEvent::Toggle
        | souvlaki::MediaControlEvent::Play
        | souvlaki::MediaControlEvent::Pause => {
            sender.send(SoundAction::PlayPause).unwrap();
        }
        souvlaki::MediaControlEvent::Next => {
            sender.send(SoundAction::Next(1)).unwrap();
        }
        souvlaki::MediaControlEvent::Previous => {
            sender.send(SoundAction::Previous(1)).unwrap();
        }
        souvlaki::MediaControlEvent::Stop => {
            sender.send(SoundAction::Cleanup).unwrap();
        }
        souvlaki::MediaControlEvent::Seek(a) => match a {
            souvlaki::SeekDirection::Forward => {
                sender.send(SoundAction::Forward).unwrap();
            }
            souvlaki::SeekDirection::Backward => {
                sender.send(SoundAction::Backward).unwrap();
            }
        },
        souvlaki::MediaControlEvent::SeekBy(_, _) => todo!(),
        souvlaki::MediaControlEvent::SetPosition(_) => todo!(),
        souvlaki::MediaControlEvent::OpenUri(_) => todo!(),
        souvlaki::MediaControlEvent::Raise => todo!(),
        souvlaki::MediaControlEvent::Quit => {
            exit(0);
        }
    })
}

pub fn get_action(
    mut index: usize,
    queue: &VecDeque<Video>,
    previous: &[Video],
    current: &Option<Video>,
) -> Option<MusicStatusAction> {
    let dw_len = IN_DOWNLOAD.lock().unwrap().len();
    if index < dw_len {
        return Some(MusicStatusAction::Downloading);
    }
    index -= dw_len;
    let previous_len = previous.len().min(3);
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
        return None;
    }
    return Some(MusicStatusAction::Skip(index + 1));
}

pub fn generate_music<'a>(
    lines: usize,
    queue: &'a VecDeque<Video>,
    previous: &'a [Video],
    current: &'a Option<Video>,
    sink: &'a Player,
) -> Vec<ListItem<'a>> {
    let download_style: Style = Style::default()
        .fg(MusicStatus::Downloading.colors().0)
        .bg(MusicStatus::Downloading.colors().1);

    let previous_style: Style = Style::default()
        .fg(MusicStatus::Previous.colors().0)
        .bg(MusicStatus::Previous.colors().1);

    let paused_style: Style = Style::default()
        .fg(MusicStatus::Paused.colors().0)
        .bg(MusicStatus::Paused.colors().1);

    let playing_style: Style = Style::default()
        .fg(MusicStatus::Playing.colors().0)
        .bg(MusicStatus::Playing.colors().1);
    let next_style: Style = Style::default()
        .fg(MusicStatus::Next.colors().0)
        .bg(MusicStatus::Next.colors().1);
    let mut music = Vec::with_capacity(50);
    {
        music.extend(IN_DOWNLOAD.lock().unwrap().iter().map(|e| {
            ListItem::new(format!(
                " {} {} | {}",
                MusicStatus::Downloading.character(),
                e.author,
                e.title
            ))
            .style(download_style)
        }));
        music.extend(previous.iter().rev().take(3).rev().map(|e| {
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
        music.extend(queue.iter().take(lines + 4).map(|e| {
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
