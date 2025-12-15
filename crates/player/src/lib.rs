mod error;

use std::time::Duration;

pub use error::PlayError;

mod player;
pub use player::Player;

mod player_options;
pub use player_options::PlayerOptions;

mod player_data;
pub(crate) use player_data::PlayerData;

pub(crate) static VOLUME_STEP: u8 = 5;
pub(crate) static SEEK_STEP: Duration = Duration::from_secs(5);
