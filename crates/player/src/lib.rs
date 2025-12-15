// Remove explicit 'cpal' crate import to avoid version mismatch.
// usage: use rodio::cpal...
use flume::Sender;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
// We import cpal traits from INSIDE rodio to ensure version compatibility
use rodio::cpal::traits::HostTrait;

use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};

static VOLUME_STEP: u8 = 5;

// Custom Error Enum to handle different failures
#[derive(Debug)]
pub enum PlayError {
    Io(std::io::Error),
    DecoderError(rodio::decoder::DecoderError),
    StreamError(rodio::StreamError),
    PlayError(rodio::PlayError),
}

impl From<rodio::PlayError> for PlayError {
    fn from(err: rodio::PlayError) -> Self {
        PlayError::PlayError(err)
    }
}

pub struct Player {
    sink: Sink,
    stream: OutputStream,
    data: PlayerData,
    error_sender: Sender<String>,
    options: PlayerOptions,
    timer: PlayTimer,
}

pub struct Guard {
    _stream: OutputStream,
}

#[derive(Clone)]
pub struct PlayerData {
    total_duration: Option<Duration>,
    volume: u8,
    safe_guard: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerOptions {
    pub initial_volume: u8,
}

/// Helper struct to track elapsed time manually
struct PlayTimer {
    start_time: Option<Instant>,
    accumulated_time: Duration,
}

impl PlayTimer {
    fn new() -> Self {
        Self {
            start_time: None,
            accumulated_time: Duration::ZERO,
        }
    }

    fn start(&mut self) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }

    fn pause(&mut self) {
        if let Some(start) = self.start_time {
            self.accumulated_time += start.elapsed();
            self.start_time = None;
        }
    }

    fn stop(&mut self) {
        self.start_time = None;
        self.accumulated_time = Duration::ZERO;
    }

    fn set_time(&mut self, time: Duration) {
        self.accumulated_time = time;
        if self.start_time.is_some() {
            self.start_time = Some(Instant::now());
        }
    }

    fn elapsed(&self) -> Duration {
        match self.start_time {
            Some(start) => self.accumulated_time + start.elapsed(),
            None => self.accumulated_time,
        }
    }
}

impl Player {
    /// Try to create a stream from a specific CPAL device
    /// Note: We use rodio::cpal::Device to match rodio's dependency version
    fn try_from_device(device: rodio::cpal::Device) -> Result<OutputStream, PlayError> {
        // In rodio 0.21, try_from_device is available on OutputStream
        OutputStreamBuilder::default()
            .with_device(device)
            .open_stream()
            .map_err(PlayError::StreamError)
    }

    /// Try to create a stream from the default device, falling back to others
    fn try_default() -> Result<OutputStream, PlayError> {
        // Use rodio's internal cpal re-export
        let host = rodio::cpal::default_host();

        let default_device = host
            .default_output_device()
            .ok_or(PlayError::StreamError(rodio::StreamError::NoDevice))?;

        Self::try_from_device(default_device).or_else(|original_err| {
            let devices = host.output_devices().map_err(|_| original_err)?;

            for d in devices {
                if let Ok(res) = Self::try_from_device(d) {
                    return Ok(res);
                }
            }
            Err(PlayError::StreamError(rodio::StreamError::NoDevice))
        })
    }

    pub fn new(error_sender: Sender<String>, options: PlayerOptions) -> Result<Self, PlayError> {
        let stream = Self::try_default()?;

        // sink::try_new requires a reference to the handle
        let sink = Sink::connect_new(stream.mixer());

        let volume = options.initial_volume.min(100);
        sink.set_volume(f32::from(volume) / 100.0);

        Ok(Self {
            sink,
            stream,
            error_sender,
            data: PlayerData {
                total_duration: None,
                volume,
                safe_guard: false,
            },
            options,
            timer: PlayTimer::new(),
        })
    }

    pub fn update(&self) -> Result<Self, PlayError> {
        let stream = Self::try_default()?;
        let sink = Sink::connect_new(stream.mixer());

        let volume = self.data.volume;
        sink.set_volume(f32::from(volume) / 100.0);

        Ok(Self {
            sink,
            stream,
            error_sender: self.error_sender.clone(),
            data: self.data.clone(),
            options: self.options.clone(),
            timer: PlayTimer {
                start_time: if self.timer.start_time.is_some() {
                    Some(Instant::now())
                } else {
                    None
                },
                accumulated_time: self.timer.elapsed(),
            },
        })
    }
}

impl Player {
    pub fn change_volume(&mut self, positive: bool) {
        if positive {
            self.data.volume = self.data.volume.saturating_add(VOLUME_STEP);
        } else {
            self.data.volume = self.data.volume.saturating_sub(VOLUME_STEP);
        }
        self.data.volume = self.data.volume.min(100);
        self.sink.set_volume(f32::from(self.data.volume) / 100.0);
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    pub fn play(&mut self, path: &Path) -> Result<(), PlayError> {
        self.stop();

        let file = File::open(path).map_err(PlayError::Io)?;

        if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            return Err(PlayError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "File is empty",
            )));
        }

        let decoder = Decoder::new(file).map_err(PlayError::DecoderError)?;

        self.data.total_duration = decoder.total_duration();

        // Check if sink is detached or empty and recreate if necessary
        if self.sink.empty() {
            // Using try_new with the stored handle
            self.sink = Sink::connect_new(self.stream.mixer());
        }

        self.sink.set_volume(f32::from(self.data.volume) / 100.0);
        self.sink.append(decoder);

        self.timer.stop();
        self.timer.start();

        Ok(())
    }

    pub fn stop(&mut self) {
        // rodio 0.21: To stop, you can clear the sink.
        if !self.sink.empty() {
            self.sink.clear();
        }
        self.timer.stop();
    }

    pub fn elapsed(&self) -> u32 {
        self.timer.elapsed().as_secs() as u32
    }

    pub fn duration(&self) -> Option<f64> {
        self.data
            .total_duration
            .map(|duration| duration.as_secs_f64())
    }

    pub fn toggle_playback(&mut self) {
        if self.sink.is_paused() {
            self.sink.play();
            self.timer.start();
        } else {
            self.sink.pause();
            self.timer.pause();
        }
    }

    pub fn seek_fw(&mut self) {
        let current_elapsed = self.timer.elapsed().as_secs_f64();
        let new_pos = current_elapsed + 5.0;

        if let Some(duration) = self.duration() {
            if new_pos > duration {
                self.data.safe_guard = true;
                self.seek_to(Duration::from_secs_f64(duration));
            } else {
                self.seek_to(Duration::from_secs_f64(new_pos));
            }
        }
    }

    pub fn seek_bw(&mut self) {
        let current_elapsed = self.timer.elapsed().as_secs_f64();
        let mut new_pos = current_elapsed - 5.0;
        if new_pos < 0.0 {
            new_pos = 0.0;
        }
        self.seek_to(Duration::from_secs_f64(new_pos));
    }

    pub fn seek_to(&mut self, time: Duration) {
        if self.sink.empty() {
            return;
        }
        if let Err(e) = self.sink.try_seek(time) {
            let _ = self.error_sender.send(format!("Seek error: {}", e));
        } else {
            self.timer.set_time(time);
        }
    }

    pub fn percentage(&self) -> f64 {
        self.duration().map_or(0.0, |duration| {
            let elapsed = self.timer.elapsed().as_secs_f64();
            elapsed / duration
        })
    }

    pub fn volume_percent(&self) -> u8 {
        self.data.volume
    }
}

// Wrapper methods
impl Player {
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

    pub fn set_volume(&mut self, mut volume: i32) {
        volume = volume.clamp(0, 100);
        self.data.volume = volume as u8;
        self.sink.set_volume((volume as f32) / 100.0);
    }

    pub fn pause(&mut self) {
        if !self.sink.is_paused() {
            self.toggle_playback();
        }
    }

    pub fn resume(&mut self) {
        if self.sink.is_paused() {
            self.toggle_playback();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn seek(&mut self, secs: i64) {
        if secs.is_positive() {
            self.seek_fw();
        } else {
            self.seek_bw();
        }
    }

    pub fn get_progress(&self) -> (f64, u32, u32) {
        let position = self.elapsed();
        let duration = self.duration().unwrap_or(99.0) as u32;
        let mut percent = self.percentage() * 100.0;
        if percent > 100.0 {
            percent = 100.0;
        }
        (percent, position, duration)
    }
}
