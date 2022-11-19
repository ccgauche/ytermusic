use std::{fs::OpenOptions, sync::RwLock};

use once_cell::sync::Lazy;
use ytpapi::Video;

mod reader;
mod writer;

pub use reader::read;
pub use writer::{write, write_video};

use crate::{consts::CACHE_DIR, systems::logger::log_};

// A global variable to store the current musical Database
pub static DATABASE: Lazy<RwLock<Vec<Video>>> = Lazy::new(|| RwLock::new(Vec::new()));

/**
 * append a video to the database
 */
pub fn append(video: Video) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(CACHE_DIR.join("db.bin"))
        .unwrap();
    write_video(&mut file, &video);
    log_(format!("Appended {} to database", video.title));
    DATABASE.write().unwrap().push(video);
}
