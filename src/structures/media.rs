use std::sync::Arc;

use flume::Sender;
use player::Player;
use souvlaki::{
    Error, MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition,
};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{consts::CONFIG, shutdown, systems::logger::log_, term::ManagerMessage};

use super::sound_action::SoundAction;

pub struct Media {
    controls: Option<MediaControls>,

    current_meta: Option<(String, String, String)>,
    current_playback: Option<MediaPlayback>,

}

impl Media {
    pub fn new(
        updater: Arc<Sender<ManagerMessage>>,
        soundaction_sender: Arc<Sender<SoundAction>>,
    ) -> Self {
        if !CONFIG.player.dbus {
            log_("[INFO] Media controls disabled by config");
            return Self {
                controls: None,
                current_meta: None,
                current_playback: None,
            };
        }
        let mut handle = get_handle(&updater);
        if let Some(e) = handle.as_mut() {
            if let Err(e) = connect(e, soundaction_sender) {
                log_(format!(
                    "[ERROR] Media actions are not supported on this platform: {e:?}",
                ));
            }
        } else {
            log_("[ERROR] Media controls are not supported on this platform");
        }
        Self {
            controls: handle,
            current_meta: None,
            current_playback: None,
        }
    }

    pub fn update(
        &mut self,
        current: &Option<YoutubeMusicVideoRef>,
        sink: &Player,
    ) -> Result<(), souvlaki::Error> {
        if let Some(e) = &mut self.controls {
            let media_meta = MediaMetadata {
                title: current.as_ref().map(|video| video.title.as_str()),
                album: current.as_ref().map(|video| video.album.as_str()),
                artist: current.as_ref().map(|video| video.author.as_str()),
                cover_url: None,
                duration: None,
            };
            if self.current_meta != Some((media_meta.title.unwrap_or("").to_string(), media_meta.album.unwrap_or("").to_string(), media_meta.artist.unwrap_or("").to_string())) {
                self.current_meta = Some((media_meta.title.unwrap_or("").to_string(), media_meta.album.unwrap_or("").to_string(), media_meta.artist.unwrap_or("").to_string()));
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

fn connect(mpris: &mut MediaControls, sender: Arc<Sender<SoundAction>>) -> Result<(), Error> {
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
        MediaControlEvent::SeekBy(_, _) => todo!(),
        MediaControlEvent::SetPosition(_) => todo!(),
        MediaControlEvent::OpenUri(_) => todo!(),
        MediaControlEvent::Raise => todo!(),
        MediaControlEvent::Quit => {
            shutdown();
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
