use std::{collections::VecDeque, path::PathBuf, process::exit, str::FromStr, sync::Arc};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, Player, StreamError};
use souvlaki::{Error, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};

use tui::{style::Style, widgets::ListItem};
use ytpapi::Video;

use crate::{
    term::{
        music_player::{MusicStatus, MusicStatusAction},
        ManagerMessage, Screens,
    },
    SoundAction,
};

use super::{
    download::{DOWNLOAD_MORE, IN_DOWNLOAD},
    logger::log,
};

#[cfg(not(target_os = "windows"))]
fn get_handle() -> Option<MediaControls> {
    handle_error_option(
        "Can't create media controls",
        MediaControls::new(PlatformConfig {
            dbus_name: "ytermusic",
            display_name: "YTerMusic",
            hwnd: None,
        }),
    )
}

#[cfg(target_os = "windows")]
fn get_handle() -> Option<MediaControls> {
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
    use winit::event_loop::EventLoop;
    use winit::{platform::windows::EventLoopExtWindows, window::WindowBuilder};

    let config = PlatformConfig {
        dbus_name: "ytermusic",
        display_name: "YTerMusic",
        hwnd: if let RawWindowHandle::Win32(h) = handle_error_option(
            "OS Error while creating media hook window",
            WindowBuilder::new()
                .with_visible(false)
                .build(&EventLoop::<()>::new_any_thread()),
        )?
        .raw_window_handle()
        {
            Some(h.hwnd)
        } else {
            log("No window handle found");
            return None;
        },
    };

    handle_error_option("Can't create media controls", MediaControls::new(config))
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
            "player creation error",
            Player::new(Arc::new(stream_error_sender)),
        )
        .unwrap();
        let mut controls = get_handle();
        if let Some(e) = &mut controls {
            handle_error(
                "Can't connect media control",
                connect(e, soundaction_sender.clone()),
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
        DOWNLOAD_MORE.store(self.queue.len() < 30, std::sync::atomic::Ordering::SeqCst);
        while let Ok(e) = self.soundaction_receiver.try_recv() {
            self.apply_sound_action(e);
        }
        if self.sink.is_finished() {
            self.handle_stream_errors();
            self.update_controls();
            if let Some(video) = self.queue.pop_front() {
                let k =
                    PathBuf::from_str(&format!("data/downloads/{}.mp4", video.video_id)).unwrap();
                if let Some(e) = self.current.replace(video) {
                    self.previous.push(e);
                }
                self.sink.play(k.as_path(), &self.guard);
            } else {
                if let Some(e) = self.current.take() {
                    self.previous.push(e);
                }
            }
        }
    }

    fn handle_stream_errors(&self) {
        while let Ok(e) = self.stream_error_receiver.try_recv() {
            self.updater
                .send(ManagerMessage::ChangeState(Screens::DeviceLost))
                .unwrap();
            log(format!("{:?}", e));
        }
    }
    fn update_controls(&mut self) {
        handle_error::<Error>(
            "Can't update finished media control",
            try {
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
            },
        );
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
                handle_error("sink stop", self.sink.stop(&self.guard));
            }
            SoundAction::Plus => self.sink.volume_up(),
            SoundAction::Minus => self.sink.volume_down(),
            SoundAction::Next(a) => {
                for _ in 1..a {
                    if let Some(e) = self.queue.pop_front() {
                        self.previous.push(e);
                    }
                }

                handle_error("sink stop", self.sink.stop(&self.guard));
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
                handle_error("sink stop", self.sink.stop(&self.guard));
            }
            SoundAction::RestartPlayer => {
                (self.sink, self.guard) =
                    handle_error_option("update player", self.sink.update()).unwrap();
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

fn handle_error_option<T, E>(error_type: &'static str, a: Result<E, T>) -> Option<E>
where
    T: std::fmt::Debug,
{
    match a {
        Ok(e) => Some(e),
        Err(a) => {
            log(format!("{}{:?}", error_type, a));
            None
        }
    }
}
fn handle_error<T>(error_type: &'static str, a: Result<(), T>)
where
    T: std::fmt::Debug,
{
    let _ = handle_error_option(error_type, a);
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
        music.extend(queue.iter().take(40).map(|e| {
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
