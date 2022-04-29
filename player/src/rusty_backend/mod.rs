#![cfg_attr(test, deny(missing_docs))]

mod conversions;
mod sink;
mod stream;

pub mod buffer;
pub mod decoder;
pub mod dynamic_mixer;
pub mod queue;
pub mod source;

pub use conversions::Sample;
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
pub use decoder::Decoder;
pub use sink::Sink;
pub use source::Source;
pub use stream::{OutputStream, OutputStreamHandle, PlayError, StreamError};

use std::path::Path;
use std::time::Duration;
use std::{fs::File, io::BufReader};

static VOLUME_STEP: u16 = 5;

pub struct Player {
    sink: Sink,
    data: PlayerData,
}

pub struct Guard {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

pub struct PlayerData {
    total_duration: Option<Duration>,
    volume: u16,
    safe_guard: bool,
}
impl Player {
    pub fn new() -> (Self, Guard) {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        let volume = 50;
        sink.set_volume(f32::from(volume) / 100.0);

        (
            Self {
                sink: sink,
                data: PlayerData {
                    total_duration: None,
                    volume,
                    safe_guard: false,
                },
            },
            Guard {
                _stream: stream,
                handle: handle,
            },
        )
    }
}

#[allow(unused)]
impl Player {
    pub fn change_volume(&mut self, positive: bool) {
        if positive {
            self.data.volume += VOLUME_STEP;
        } else if self.data.volume >= VOLUME_STEP {
            self.data.volume -= VOLUME_STEP;
        } else {
            self.data.volume = 0;
        }
        self.data.volume = self.data.volume.min(100);
        self.sink.set_volume(f32::from(self.data.volume) / 100.0);
    }
    pub fn is_finished(&self) -> bool {
        self.sink.is_empty() || self.sink.sleep_until_end()
    }
    pub fn play(&mut self, path: &Path, guard: &Guard) {
        self.stop(guard);
        let file = File::open(path).unwrap();
        //println!("{:?}", path);
        let decoder = Decoder::new_decoder(BufReader::new(file)).unwrap();
        self.data.total_duration = decoder.total_duration();
        self.sink.append(decoder);
    }
    pub fn stop(&mut self, guard: &Guard) {
        self.sink.destroy();
        self.sink = Sink::try_new(&guard.handle).unwrap();
        self.sink.set_volume(f32::from(self.data.volume) / 100.0);
    }
    pub fn elapsed(&self) -> Duration {
        self.sink.elapsed()
    }
    pub fn duration(&self) -> Option<f64> {
        self.data
            .total_duration
            .map(|duration| duration.as_secs_f64() - 0.29)
    }
    pub fn toggle_playback(&self) {
        self.sink.toggle_playback();
    }
    pub fn seek_fw(&mut self) {
        let new_pos = self.elapsed().as_secs_f64() + 5.0;
        if let Some(duration) = self.duration() {
            if new_pos > duration {
                self.data.safe_guard = true;
            } else {
                self.seek_to(Duration::from_secs_f64(new_pos));
            }
        }
    }
    pub fn seek_bw(&self) {
        let mut new_pos = self.elapsed().as_secs_f64() - 5.0;
        if new_pos < 0.0 {
            new_pos = 0.0;
        }

        self.seek_to(Duration::from_secs_f64(new_pos));
    }
    pub fn seek_to(&self, time: Duration) {
        self.sink.seek(time);
    }
    pub fn percentage(&self) -> f64 {
        self.duration().map_or(0.0, |duration| {
            let elapsed = self.elapsed();
            elapsed.as_secs_f64() / duration
        })
    }
    pub fn volume_percent(&self) -> u16 {
        self.data.volume
    }
}

impl Player {
    pub fn add_and_play(&mut self, song: &str, guard: &Guard) {
        let p = Path::new(song);
        self.play(p, guard);
    }

    pub fn volume(&self) -> i32 {
        self.data.volume.into()
    }

    pub fn volume_up(&mut self) {
        let volume = self.volume() + 5;
        self.set_volume(volume);
    }

    pub fn volume_down(&mut self) {
        let volume = self.volume() - 5;
        self.set_volume(volume);
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn set_volume(&mut self, mut volume: i32) {
        if volume > 100 {
            volume = 100;
        } else if volume < 0 {
            volume = 0;
        }
        self.data.volume = volume as u16;
        self.sink.set_volume((volume as f32) / 100.0);
    }

    pub fn pause(&self) {
        self.toggle_playback();
    }

    pub fn resume(&self) {
        self.toggle_playback();
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn seek(&mut self, secs: i64) {
        if secs.is_positive() {
            self.seek_fw();
            return;
        }
        self.seek_bw();
    }

    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
    pub fn get_progress(&self) -> (f64, i64, i64) {
        let position = self.elapsed().as_secs() as i64;
        let duration = self.duration().unwrap_or(99.0) as i64;
        let mut percent = self.percentage() * 100.0;
        if percent > 100.0 {
            percent = 100.0;
        }
        (percent, position, duration)
    }
}
