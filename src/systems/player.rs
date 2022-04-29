use std::{
    collections::VecDeque, path::PathBuf, process::exit, str::FromStr, sync::Arc, time::Duration,
};

use flume::Sender;
use player::{Guard, Player};
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

pub fn player_system(updater: Arc<Sender<ManagerMessage>>) -> Arc<Sender<SoundAction>> {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    let tx = Arc::new(tx);
    let k = tx.clone();
    std::thread::spawn(move || {
        let (mut sink, guard) = Player::new();
        let mut queue: VecDeque<Video> = VecDeque::new();
        let mut previous: Vec<Video> = Vec::new();
        let mut current: Option<Video> = None;
        let mut controls = get_handle();
        if let Some(e) = &mut controls {
            handle_error("Can't connect media control", connect(e, k.clone()));
        }

        loop {
            if let Some(e) = &mut controls {
                handle_error("Can't update media control", update(e, &sink, &current));
            }
            DOWNLOAD_MORE.store(queue.len() < 30, std::sync::atomic::Ordering::SeqCst);
            updater
                .send(ManagerMessage::PassTo(
                    Screens::MusicPlayer,
                    Box::new(ManagerMessage::UpdateApp(App::new(
                        &sink,
                        generate_music(&queue, &previous, &current, &sink),
                        k.clone(),
                    ))),
                ))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
            while let Ok(e) = rx.try_recv() {
                apply_sound_action(
                    e,
                    &mut sink,
                    &guard,
                    &mut queue,
                    &mut previous,
                    &mut current,
                );
            }
            if sink.is_finished() {
                'a: loop {
                    if let Some(e) = &mut controls {
                        handle_error(
                            "Can't update finished media control",
                            update(e, &sink, &current),
                        );
                    }
                    if let Some(video) = queue.pop_front() {
                        let k =
                            PathBuf::from_str(&format!("data/downloads/{}.mp4", video.video_id))
                                .unwrap();
                        if let Some(e) = current.replace(video) {
                            previous.push(e);
                        }
                        sink.play(k.as_path(), &guard);
                        break 'a;
                    } else {
                        if let Some(e) = current.take() {
                            previous.push(e);
                        }
                        while let Ok(e) = rx.try_recv() {
                            apply_sound_action(
                                e.clone(),
                                &mut sink,
                                &guard,
                                &mut queue,
                                &mut previous,
                                &mut current,
                            );
                            if matches!(e, SoundAction::PlayVideo(_)) {
                                continue 'a;
                            }
                        }
                        std::thread::sleep(Duration::from_millis(200));
                        updater
                            .send(ManagerMessage::PassTo(
                                Screens::MusicPlayer,
                                Box::new(ManagerMessage::UpdateApp(App::new(
                                    &sink,
                                    generate_music(&queue, &previous, &current, &sink),
                                    k.clone(),
                                ))),
                            ))
                            .unwrap();
                    }
                }
            }
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

fn update(e: &mut MediaControls, sink: &Player, current: &Option<Video>) -> Result<(), Error> {
    e.set_metadata(MediaMetadata {
        title: current.as_ref().map(|video| video.title.as_str()),
        album: current.as_ref().map(|video| video.album.as_str()),
        artist: current.as_ref().map(|video| video.author.as_str()),
        cover_url: None,
        duration: None,
    })?;
    if sink.is_finished() {
        e.set_playback(MediaPlayback::Stopped)?;
    } else if sink.is_paused() {
        e.set_playback(MediaPlayback::Paused {
            progress: Some(MediaPosition(sink.elapsed())),
        })?;
    } else {
        e.set_playback(MediaPlayback::Playing {
            progress: Some(MediaPosition(sink.elapsed())),
        })?;
    }
    Ok(())
}

fn apply_sound_action(
    e: SoundAction,
    sink: &mut Player,
    guard: &Guard,
    queue: &mut VecDeque<Video>,
    previous: &mut Vec<Video>,
    current: &mut Option<Video>,
) {
    match e {
        SoundAction::Backward => sink.seek_bw(),
        SoundAction::Forward => sink.seek_fw(),
        SoundAction::PlayPause => sink.toggle_playback(),
        SoundAction::Cleanup => {
            queue.clear();
            previous.clear();
            *current = None;
            sink.stop(guard);
        }
        SoundAction::Plus => sink.volume_up(),
        SoundAction::Minus => sink.volume_down(),
        SoundAction::Next(a) => {
            /* if sink.is_finished() {
                if let Some(e) = queue.pop_front() {
                    previous.push(e);
                }
            } */
            for _ in 1..a {
                if let Some(e) = queue.pop_front() {
                    previous.push(e);
                }
            }

            sink.stop(guard);
        }
        SoundAction::PlayVideo(video) => {
            queue.push_back(video);
        }
        SoundAction::Previous(a) => {
            for _ in 0..a {
                if let Some(e) = previous.pop() {
                    if let Some(c) = current.take() {
                        queue.push_front(c);
                    }
                    queue.push_front(e);
                }
            }
            sink.stop(guard);
        }
    }
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
