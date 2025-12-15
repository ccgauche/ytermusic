use std::{path::PathBuf, time::Duration};

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

    pub fn volume(&self) -> u8 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    pub fn set_total_duration(&mut self, total_duration: Option<Duration>) {
        self.total_duration = total_duration;
    }

    pub fn current_file(&self) -> Option<PathBuf> {
        self.current_file.clone()
    }

    pub fn set_current_file(&mut self, current_file: Option<PathBuf>) {
        self.current_file = current_file;
    }
}
