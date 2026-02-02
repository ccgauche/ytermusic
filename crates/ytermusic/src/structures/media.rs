use std::time::Duration;

use flume::Sender;
use log::{error, info};
use player::Player;
use souvlaki::{
    Error, MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition,
    SeekDirection,
};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{consts::CONFIG, shutdown, term::ManagerMessage};

use super::sound_action::SoundAction;

pub struct Media {
    controls: Option<MediaControls>,

    current_meta: Option<(String, String, String, Option<Duration>)>,
    current_playback: Option<MediaPlayback>,
}

impl Media {
    pub fn new(updater: Sender<ManagerMessage>, soundaction_sender: Sender<SoundAction>) -> Self {
        if !CONFIG.player.dbus {
            info!("Media controls disabled by config");
            return Self {
                controls: None,
                current_meta: None,
                current_playback: None,
            };
        }
        let mut handle = get_handle(&updater);
        if let Some(e) = handle.as_mut() {
            if let Err(e) = connect(e, soundaction_sender) {
                error!("Media actions are not supported on this platform: {e:?}",);
            }
        } else {
            error!("Media controls are not supported on this platform");
        }
        Self {
            controls: handle,
            current_meta: None,
            current_playback: None,
        }
    }

    pub fn update(
        &mut self,
        current: Option<YoutubeMusicVideoRef>,
        sink: &Player,
    ) -> Result<(), souvlaki::Error> {
        if let Some(e) = &mut self.controls {
            let media_meta = MediaMetadata {
                title: current.as_ref().map(|video| video.title.as_str()),
                album: current.as_ref().map(|video| video.album.as_str()),
                artist: current.as_ref().map(|video| video.author.as_str()),
                cover_url: None,
                duration: sink
                    .duration()
                    .map(|duration| Duration::from_secs(duration as u64)),
            };
            if self.current_meta
                != Some((
                    media_meta.title.unwrap_or("").to_string(),
                    media_meta.album.unwrap_or("").to_string(),
                    media_meta.artist.unwrap_or("").to_string(),
                    sink.duration()
                        .map(|duration| Duration::from_secs(duration as u64)),
                ))
            {
                self.current_meta = Some((
                    media_meta.title.unwrap_or("").to_string(),
                    media_meta.album.unwrap_or("").to_string(),
                    media_meta.artist.unwrap_or("").to_string(),
                    sink.duration()
                        .map(|duration| Duration::from_secs(duration as u64)),
                ));
                e.set_metadata(media_meta)?;
            }
            let playback = if sink.is_finished() {
                MediaPlayback::Stopped
            } else if sink.is_paused() {
                MediaPlayback::Paused {
                    progress: Some(MediaPosition(sink.elapsed())),
                }
            } else {
                MediaPlayback::Playing {
                    progress: Some(MediaPosition(sink.elapsed())),
                }
            };
            if self.current_playback != Some(playback.clone()) {
                self.current_playback = Some(playback.clone());
                e.set_playback(playback)?;
            }
        }
        Ok(())
    }
}

fn connect(mpris: &mut MediaControls, sender: Sender<SoundAction>) -> Result<(), Error> {
    mpris.attach(move |e| match e {
        MediaControlEvent::Toggle | MediaControlEvent::Play | MediaControlEvent::Pause => {
            sender.send(SoundAction::PlayPause).unwrap();
        }
        MediaControlEvent::Next => {
            sender.send(SoundAction::Next(1)).unwrap();
        }
        MediaControlEvent::Previous => {
            sender.send(SoundAction::Previous(1)).unwrap();
        }
        MediaControlEvent::Stop => {
            sender.send(SoundAction::Cleanup).unwrap();
        }
        MediaControlEvent::Seek(a) => match a {
            souvlaki::SeekDirection::Forward => {
                sender.send(SoundAction::Forward).unwrap();
            }
            souvlaki::SeekDirection::Backward => {
                sender.send(SoundAction::Backward).unwrap();
            }
        },
        // TODO(functionnality): implement seek amount
        MediaControlEvent::SeekBy(a, _b) => {
            if a == SeekDirection::Forward {
                sender.send(SoundAction::Forward).unwrap();
            } else {
                sender.send(SoundAction::Backward).unwrap();
            }
        }

        MediaControlEvent::SetPosition(a) => {
            sender.send(SoundAction::SeekTo(a.0)).unwrap();
        }
        MediaControlEvent::OpenUri(a) => {
            todo!("Implement URI opening {a:?}")
        }
        MediaControlEvent::Raise => {
            todo!("Implement raise")
        }
        MediaControlEvent::Quit => {
            shutdown();
        }
        MediaControlEvent::SetVolume(e) => {
            sender.send(SoundAction::SetVolume(e as f32)).unwrap();
        }
    })
}

#[cfg(not(target_os = "windows"))]
fn get_handle(updater: &Sender<ManagerMessage>) -> Option<MediaControls> {
    use crate::errors::handle_error_option;
    use souvlaki::PlatformConfig;
    handle_error_option(
        updater,
        "Can't create media controls",
        MediaControls::new(PlatformConfig {
            dbus_name: "ytermusic",
            display_name: "YTerMusic",
            hwnd: None,
        })
        .map_err(|e| format!("{e:?}")),
    )
}
#[cfg(not(target_os = "macos"))]
pub fn run_window_handler(_updater: &Sender<ManagerMessage>) -> Option<()> {
    use crate::is_shutdown_sent;

    loop {
        if !is_shutdown_sent() {
            crate::shutdown::block_until_shutdown()
        } else {
            use std::process::exit;

            info!("event loop closed");
            exit(0);
        }
    }
}

#[cfg(target_os = "macos")]
pub fn run_window_handler(updater: &Sender<ManagerMessage>) -> Option<()> {
    use std::process::exit;

    use winit::event_loop::EventLoop;
    use winit::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
    use winit::window::WindowBuilder;

    use crate::errors::handle_error_option;
    let thread = std::thread::current();
    info!("Current Thread Name: {:?}", thread.name());
    info!("Current Thread ID:   {:?}", thread.id());

    // On macOS, winit requires the EventLoop to be created on the main thread.
    // Unlike Windows, we cannot use `new_any_thread`.
    // We create a hidden window to ensure NSApplication is active and capable of receiving events.
    let mut event_loop = EventLoop::new();
    event_loop.set_activation_policy(ActivationPolicy::Regular);

    // Create a hidden window. While souvlaki doesn't need the handle in the config,
    // the existence of the window helps keep the event loop and application state valid.
    let _window = handle_error_option(
        updater,
        "OS Error while creating media hook window",
        WindowBuilder::new().with_visible(false).build(&event_loop),
    )?;
    event_loop.run(move |_event, _window_target, ctrl_flow| {
        use crate::is_shutdown_sent;

        if is_shutdown_sent() {
            info!("event loop closed");
            *ctrl_flow = winit::event_loop::ControlFlow::Exit;
            exit(0);
        }
    });
}

#[cfg(target_os = "windows")]
fn get_handle(updater: &Sender<ManagerMessage>) -> Option<MediaControls> {
    use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
    use souvlaki::PlatformConfig;
    use winit::event_loop::EventLoop;
    use winit::{platform::windows::EventLoopExtWindows, window::WindowBuilder};

    use crate::errors::handle_error_option;
    use crate::term::Screens;

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
                    Box::new(ManagerMessage::Error(
                        "No window handle found".to_string(),
                        Box::new(None),
                    )),
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
