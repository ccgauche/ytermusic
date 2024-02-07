use std::{fs::OpenOptions, sync::RwLock};

use log::info;
use once_cell::sync::Lazy;

mod reader;
mod writer;

pub use reader::read;
pub use writer::{write, write_video};
use ytpapi2::YoutubeMusicVideoRef;

use crate::consts::CACHE_DIR;

/// A global variable to store the current musical Database
pub static DATABASE: Lazy<RwLock<Vec<YoutubeMusicVideoRef>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

/// Remove a video from the database
pub fn remove_video(video: &YoutubeMusicVideoRef) {
    let mut database = DATABASE.write().unwrap();
    database.retain(|v| v.video_id != video.video_id);
    write();
}

/// Append a video to the database
pub fn append(video: YoutubeMusicVideoRef) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(CACHE_DIR.join("db.bin"))
        .unwrap();
    write_video(&mut file, &video);
    info!("Appended {} to database", video.title);
    DATABASE.write().unwrap().push(video);
}
