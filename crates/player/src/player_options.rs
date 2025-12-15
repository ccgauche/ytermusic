#[derive(Debug, Clone)]
pub struct PlayerOptions {
    initial_volume: u8,
}

impl PlayerOptions {
    /// Creates a new PlayerOptions with the given initial volume
    pub fn new(initial_volume: u8) -> Self {
        Self {
            initial_volume: initial_volume.min(100),
        }
    }

    /// Returns the initial volume as a u8 between 0 and 100
    pub fn initial_volume(&self) -> u8 {
        self.initial_volume
    }

    /// Returns the initial volume as a f32 between 0.0 and 1.0
    pub fn initial_volume_f32(&self) -> f32 {
        f32::from(self.initial_volume()) / 100.0
    }
}
