use std::{fs::OpenOptions, io::Write};

use varuint::WriteVarint;
use ytpapi::Video;

use crate::consts::CACHE_DIR;

/**
 * Writes the database to the disk
 */
pub fn write() {
    let db = super::DATABASE.read().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open(CACHE_DIR.join("db.bin"))
        .unwrap();
    for video in db.iter() {
        write_video(&mut file, video)
    }
}

/**
 * Writes a video to a file
 */
pub fn write_video(buffer: &mut impl Write, video: &Video) {
    write_str(buffer, &video.title);
    write_str(buffer, &video.author);
    write_str(buffer, &video.album);
    write_str(buffer, &video.video_id);
    write_str(buffer, &video.duration);
}

/**
 * Writes a string from the cursor
 */
fn write_str(cursor: &mut impl Write, value: &str) {
    write_u32(cursor, value.len() as u32);
    cursor.write_all(value.as_bytes()).unwrap();
}

/**
 * Writes a u32 from the cursor
 */
fn write_u32(cursor: &mut impl Write, value: u32) {
    cursor.write_varint(value).unwrap();
}
