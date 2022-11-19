#![feature(try_blocks)]
#![feature(cursor_remaining)]

use once_cell::sync::Lazy;
use rustube::Error;
use term::{Manager, ManagerMessage, Screens};
use varuint::{ReadVarint, WriteVarint};

use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{Cursor, Read, Write};
use std::sync::RwLock;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use systems::download::downloader;
use systems::player::player_system;

use ytpapi::{Video, YTApi};

use crate::systems::logger::log_;

mod errors;
mod systems;
mod term;

use mimalloc::MiMalloc;

// Changes the allocator to improve performance especially on Windows
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/**
 * Actions that can be sent to the player from other services
 */
#[derive(Debug, Clone)]
pub enum SoundAction {
    Cleanup,
    PlayPause,
    ForcePause,
    ForcePlay,
    RestartPlayer,
    Plus,
    Minus,
    Previous(usize),
    Forward,
    Backward,
    Next(usize),
    PlayVideo(Video),
    PlayVideoUnary(Video),
}

// A global variable to store the current musical Database
pub static DATABASE: Lazy<RwLock<Vec<Video>>> = Lazy::new(|| RwLock::new(Vec::new()));

/**
 * Writes the database to the disk
 */
fn write() {
    let db = DATABASE.read().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .open("data/db.bin")
        .unwrap();
    for video in db.iter() {
        write_video(&mut file, video)
    }
}
/**
 * append a video to the database
 */
pub fn append(video: Video) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("data/db.bin")
        .unwrap();
    write_video(&mut file, &video);
    log_(format!("Appended {} to database", video.title));
    DATABASE.write().unwrap().push(video);
}

/**
 * Writes a video to a file
 */
fn write_video(buffer: &mut impl Write, video: &Video) {
    write_str(buffer, &video.title);
    write_str(buffer, &video.author);
    write_str(buffer, &video.album);
    write_str(buffer, &video.video_id);
    write_str(buffer, &video.duration);
}

/**
 * Reads the database
 */
fn read() -> Option<Vec<Video>> {
    let mut buffer = Cursor::new(std::fs::read("data/db.bin").ok()?);
    let mut videos = HashSet::new();
    while !buffer.is_empty() {
        videos.insert(read_video(&mut buffer)?);
    }
    Some(videos.into_iter().collect::<Vec<_>>())
}

/**
 * Reads a video from the cursor
 */
fn read_video(buffer: &mut Cursor<Vec<u8>>) -> Option<Video> {
    Some(Video {
        title: read_str(buffer)?,
        author: read_str(buffer)?,
        album: read_str(buffer)?,
        video_id: read_str(buffer)?,
        duration: read_str(buffer)?,
    })
}

/**
 * Writes a string from the cursor
 */
fn write_str(cursor: &mut impl Write, value: &str) {
    write_u32(cursor, value.len() as u32);
    cursor.write_all(value.as_bytes()).unwrap();
}

/**
 * Reads a string from the cursor
 */
fn read_str(cursor: &mut Cursor<Vec<u8>>) -> Option<String> {
    let mut buf = vec![0u8; read_u32(cursor)? as usize];
    cursor.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

/**
 * Writes a u32 from the cursor
 */
fn write_u32(cursor: &mut impl Write, value: u32) {
    cursor.write_varint(value).unwrap();
}

/**
 * Reads a u32 from the cursor
 */
fn read_u32(cursor: &mut Cursor<Vec<u8>>) -> Option<u32> {
    ReadVarint::<u32>::read_varint(cursor).ok()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    std::fs::create_dir_all("data/downloads").unwrap();
    if !PathBuf::from_str("headers.txt").unwrap().exists() {
        println!("The `headers.txt` file is not present in the root directory.");
        println!("To configure the YTerMusic:");
        println!("1. Open the YouTube Music website in your browser");
        println!("2. Open the developer tools (F12)");
        println!("3. Go to the Network tab");
        println!("4. Go to https://music.youtube.com");
        println!("5. Copy the `cookie` header from the associated request");
        println!("6. Paste it in the `headers.txt` file as `Cookie: <cookie>`");
        println!("7. Restart YterMusic");
        return Ok(());
    }
    if !std::fs::read_to_string("headers.txt")
        .unwrap()
        .contains("Cookie: ")
    {
        println!("The `headers.txt` file is not configured correctly.");
        println!("To configure the YTerMusic:");
        println!("1. Open the YouTube Music website in your browser");
        println!("2. Open the developer tools (F12)");
        println!("3. Go to the Network tab");
        println!("4. Go to https://music.youtube.com");
        println!("5. Copy the `cookie` header from the associated request");
        println!("6. Paste it in the `headers.txt` file as `Cookie: <cookie>`");
        println!("7. Restart YterMusic");
        return Ok(());
    }
    // Spawn the clean task
    let (updater_s, updater_r) = flume::unbounded::<ManagerMessage>();
    tokio::task::spawn(async {
        clean();
    });
    let updater_s = Arc::new(updater_s);
    // Spawn the player task
    let (sa, player) = player_system(updater_s.clone());
    // Spawn the downloader task
    downloader(sa.clone());
    {
        let updater_s = updater_s.clone();
        // Spawn playlist updater task
        tokio::task::spawn(async move {
            let playlist = std::fs::read_to_string("data/last-playlist.json").ok()?;
            let mut playlist: (String, Vec<Video>) = serde_json::from_str(&playlist).ok()?;
            if !playlist.0.starts_with("Last playlist: ") {
                playlist.0 = format!("Last playlist: {}", playlist.0);
            }
            updater_s
                .send(ManagerMessage::AddElementToChooser(playlist).pass_to(Screens::Playlist))
                .unwrap();
            Some(())
        });
    }
    {
        let updater_s = updater_s.clone();
        // Spawn the API task
        tokio::task::spawn(async move {
            match YTApi::from_header_file(PathBuf::from_str("headers.txt").unwrap().as_path()).await
            {
                Ok(api) => {
                    let api = Arc::new(api);
                    for playlist in api.playlists() {
                        let updater_s = updater_s.clone();
                        let playlist = playlist.clone();
                        let api = api.clone();
                        tokio::task::spawn(async move {
                            match api.browse_playlist(&playlist.browse_id).await {
                                Ok(videos) => {
                                    updater_s
                                        .send(
                                            ManagerMessage::AddElementToChooser((
                                                format!(
                                                    "{} ({})",
                                                    playlist.name, playlist.subtitle
                                                ),
                                                videos,
                                            ))
                                            .pass_to(Screens::Playlist),
                                        )
                                        .unwrap();
                                }
                                Err(e) => {
                                    log_(format!("{:?}", e));
                                }
                            }
                        });
                    }
                }
                Err(e) => {
                    log_(format!("{:?}", e));
                }
            }
        });
    }
    {
        let updater_s = updater_s.clone();
        // Spawn the database getter task
        tokio::task::spawn(async move {
            if let Some(e) = read() {
                *DATABASE.write().unwrap() = e.clone();

                updater_s
                    .send(
                        ManagerMessage::AddElementToChooser(("Local musics".to_owned(), e))
                            .pass_to(Screens::Playlist),
                    )
                    .unwrap();
            } else {
                let mut videos = HashSet::new();
                for files in std::fs::read_dir("data/downloads").unwrap() {
                    let path = files.unwrap().path();
                    if path.as_os_str().to_string_lossy().ends_with(".json") {
                        let video =
                            serde_json::from_str(std::fs::read_to_string(path).unwrap().as_str())
                                .unwrap();
                        videos.insert(video);
                    }
                }

                let k = videos.into_iter().collect::<Vec<_>>();

                *DATABASE.write().unwrap() = k.clone();

                updater_s
                    .send(
                        ManagerMessage::AddElementToChooser(("Local musics".to_owned(), k))
                            .pass_to(Screens::Playlist),
                    )
                    .unwrap();
                write();
            }
        });
    }
    let mut manager = Manager::new(sa, player).await;
    manager.run(&updater_r).unwrap();
    Ok(())
}

/**
 * This function is called on start to clean the database and the files that are incompletly downloaded due to a crash.
 */
fn clean() {
    for i in std::fs::read_dir("data/downloads").unwrap() {
        let path = i.unwrap().path();
        if path.ends_with(".mp4") {
            let mut path1 = path.clone();
            path1.set_extension("json");
            if !path1.exists() {
                std::fs::remove_file(&path).unwrap();
            }
        }
    }
}
