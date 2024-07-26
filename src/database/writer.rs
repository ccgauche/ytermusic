use std::{fs::OpenOptions, io::Write};

use varuint::WriteVarint;
use ytpapi2::YoutubeMusicVideoRef;

use crate::consts::CACHE_DIR;

use super::DATABASE;

/// Writes the database to the disk
pub fn write() {
    let db = super::DATABASE.read().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .truncate(true)
        .open(CACHE_DIR.join("db.bin"))
        .unwrap();
    for video in db.iter() {
        write_video(&mut file, video)
    }
}

pub fn fix_db() {
    let mut db = DATABASE.write().unwrap();
    db.clear();
    let cache_folder = CACHE_DIR.join("downloads");
    for entry in std::fs::read_dir(&cache_folder).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        // Check if the file is a json file
        if path.extension().unwrap() != "json" {
            continue;
        }
        // Read the file if not readable do not add it to the database
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                println!(
                    "[INFO] Removing file {:?} because the file is not readable: {e:?}",
                    path.file_name()
                );
                continue;
            }
        };
        // Check if the file is a valid json file
        let video = match serde_json::from_str::<YoutubeMusicVideoRef>(&content) {
            Ok(parsed) => parsed,
            Err(e) => {
                println!(
                    "[INFO] Removing file {:?} because the file is not a valid json file: {e:?}",
                    path.file_name()
                );
                continue;
            }
        };
        // Check if the video file exists
        let video_file = cache_folder.join(format!("{}.mp4", video.video_id));
        if !video_file.exists() {
            println!(
                "[INFO] Removing file {:?} because the video file does not exist",
                path.file_name()
            );
            continue;
        }
        // Read the video file
        let video_file = match std::fs::read(&video_file) {
            Ok(video_file) => video_file,
            Err(e) => {
                println!(
                    "[INFO] Removing file {:?} because the video file is not readable: {e:?}",
                    path.file_name()
                );
                continue;
            }
        };
        // Check if the video file contains the header
        if !video_file.starts_with(&[
            0, 0, 0, 24, 102, 116, 121, 112, 100, 97, 115, 104, 0, 0, 0, 0,
        ]) {
            println!(
                "[INFO] Removing file {:?} because the video file does not contain the header",
                path.file_name()
            );
            continue;
        }

        db.push(video);
    }
}

/// Writes a video to a file
pub fn write_video(buffer: &mut impl Write, video: &YoutubeMusicVideoRef) {
    write_str(buffer, &video.title);
    write_str(buffer, &video.author);
    write_str(buffer, &video.album);
    write_str(buffer, &video.video_id);
    write_str(buffer, &video.duration);
}

/// Writes a string from the cursor
fn write_str(cursor: &mut impl Write, value: &str) {
    write_u32(cursor, value.len() as u32);
    cursor.write_all(value.as_bytes()).unwrap();
}

/// Writes a u32 from the cursor
fn write_u32(cursor: &mut impl Write, value: u32) {
    cursor.write_varint(value).unwrap();
}
