use std::{
    collections::VecDeque, path::PathBuf, process::exit, str::FromStr, sync::Arc, time::Duration,
};

use flume::{unbounded, Receiver, Sender};
use player::{Guard, Player, StreamError};
use souvlaki::{Error, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};

use ytpapi::Video;

use crate::{
    term::{
        music_player::{App, MusicStatus, MusicStatusAction, UIMusic},
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

struct PlayerState {
    queue: VecDeque<Video>,
    current: Option<Video>,
    previous: Vec<Video>,
    controls: Option<MediaControls>,
    sink: Player,
    guard: Guard,
    updater: Arc<Sender<ManagerMessage>>,
    soundaction_sender: Arc<Sender<SoundAction>>,
    soundaction_receiver: Receiver<SoundAction>,
    stream_error_receiver: Receiver<StreamError>,
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

    fn update(&mut self) {
        self.update_controls();
        self.handle_stream_errors();
        DOWNLOAD_MORE.store(self.queue.len() < 30, std::sync::atomic::Ordering::SeqCst);
        self.updater
            .send(
                ManagerMessage::UpdateApp(App::new(
                    &self.sink,
                    generate_music(&self.queue, &self.previous, &self.current, &self.sink),
                    self.soundaction_sender.clone(),
                ))
                .pass_to(Screens::MusicPlayer),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        while let Ok(e) = self.soundaction_receiver.try_recv() {
            self.apply_sound_action(e);
        }
        if self.sink.is_finished() {
            'a: loop {
                self.handle_stream_errors();
                self.update_controls();
                if let Some(video) = self.queue.pop_front() {
                    let k = PathBuf::from_str(&format!("data/downloads/{}.mp4", video.video_id))
                        .unwrap();
                    if let Some(e) = self.current.replace(video) {
                        self.previous.push(e);
                    }
                    self.sink.play(k.as_path(), &self.guard);
                    break 'a;
                } else {
                    if let Some(e) = self.current.take() {
                        self.previous.push(e);
                    }
                    while let Ok(e) = self.soundaction_receiver.try_recv() {
                        self.apply_sound_action(e.clone());
                        if matches!(e, SoundAction::PlayVideo(_)) {
                            continue 'a;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(200));
                    self.updater
                        .send(
                            ManagerMessage::UpdateApp(App::new(
                                &self.sink,
                                generate_music(
                                    &self.queue,
                                    &self.previous,
                                    &self.current,
                                    &self.sink,
                                ),
                                self.soundaction_sender.clone(),
                            ))
                            .pass_to(Screens::MusicPlayer),
                        )
                        .unwrap();
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
    fn apply_sound_action(&mut self, e: SoundAction) {
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
                /* if sink.is_finished() {
                    if let Some(e) = queue.pop_front() {
                        previous.push(e);
                    }
                } */
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

pub fn player_system(updater: Arc<Sender<ManagerMessage>>) -> Arc<Sender<SoundAction>> {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    let tx = Arc::new(tx);
    let k = tx.clone();
    std::thread::spawn(move || {
        let mut state = PlayerState::new(k, rx, updater);
        loop {
            state.update();
        }
    });
    tx
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

fn generate_music(
    queue: &VecDeque<Video>,
    previous: &[Video],
    current: &Option<Video>,
    sink: &Player,
) -> Vec<UIMusic> {
    let mut music = Vec::new();
    {
        music.extend(
            IN_DOWNLOAD
                .lock()
                .unwrap()
                .iter()
                .map(|e| UIMusic::new(e, MusicStatus::Downloading, MusicStatusAction::Downloading)),
        );
        previous
            .iter()
            .rev()
            .take(3)
            .enumerate()
            .rev()
            .for_each(|e| {
                music.push(UIMusic::new(
                    e.1,
                    MusicStatus::Previous,
                    MusicStatusAction::Before(e.0 + 1),
                ));
            });
        if let Some(e) = current {
            music.push(UIMusic::new(
                e,
                if sink.is_paused() {
                    MusicStatus::Paused
                } else {
                    MusicStatus::Playing
                },
                MusicStatusAction::Current,
            ));
        }
        music.extend(
            queue
                .iter()
                .take(40)
                .enumerate()
                .map(|e| UIMusic::new(e.1, MusicStatus::Next, MusicStatusAction::Skip(e.0 + 1))),
        );
    }
    music
}
