use std::{path::PathBuf, time::Duration};

use crate::VOLUME_STEP;

#[derive(Clone)]
pub struct PlayerData {
    total_duration: Option<Duration>,
    current_file: Option<PathBuf>,
    volume: u8,
}

impl PlayerData {
    pub fn new(volume: u8) -> Self {
        Self {
            total_duration: None,
            current_file: None,
            volume,
        }
    }

    /// Changes the volume by the volume step. If positive is true, the volume is increased, otherwise it is decreased.
    pub fn change_volume(&mut self, positive: bool) {
        if positive {
            self.set_volume(self.volume().saturating_add(VOLUME_STEP).min(100));
        } else {
            self.set_volume(self.volume().saturating_sub(VOLUME_STEP));
        }
    }

    /// Returns the volume as a f32 between 0.0 and 1.0
    pub fn volume_f32(&self) -> f32 {
        f32::from(self.volume()) / 100.0
    }

    /// Returns the volume as a u8 between 0 and 100
    pub fn volume(&self) -> u8 {
        self.volume
    }

    /// Sets the volume to the given value
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    /// Returns the total duration of the current file
    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    /// Sets the total duration of the current file
    pub fn set_total_duration(&mut self, total_duration: Option<Duration>) {
        self.total_duration = total_duration;
    }

    /// Returns the current file
    pub fn current_file(&self) -> Option<PathBuf> {
        self.current_file.clone()
    }

    /// Sets the current file
    pub fn set_current_file(&mut self, current_file: Option<PathBuf>) {
        self.current_file = current_file;
    }
}
