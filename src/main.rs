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

pub static DATABASE: Lazy<RwLock<Vec<Video>>> = Lazy::new(|| RwLock::new(Vec::new()));

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
pub fn append(video: Video) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("data/db.bin")
        .unwrap();
    write_video(&mut file, &video);
    DATABASE.write().unwrap().push(video);
}

fn write_video(buffer: &mut impl Write, video: &Video) {
    write_str(buffer, &video.title);
    write_str(buffer, &video.author);
    write_str(buffer, &video.album);
    write_str(buffer, &video.video_id);
    write_str(buffer, &video.duration);
}

fn read() -> Option<Vec<Video>> {
    let mut buffer = Cursor::new(std::fs::read("data/db.bin").ok()?);
    let mut videos = HashSet::new();
    while !buffer.is_empty() {
        videos.insert(read_video(&mut buffer)?);
    }
    Some(videos.into_iter().collect::<Vec<_>>())
}

fn read_video(buffer: &mut Cursor<Vec<u8>>) -> Option<Video> {
    Some(Video {
        title: read_str(buffer)?,
        author: read_str(buffer)?,
        album: read_str(buffer)?,
        video_id: read_str(buffer)?,
        duration: read_str(buffer)?,
    })
}

fn write_str(cursor: &mut impl Write, value: &str) {
    write_u32(cursor, value.len() as u32);
    cursor.write_all(value.as_bytes()).unwrap();
}

fn read_str(cursor: &mut Cursor<Vec<u8>>) -> Option<String> {
    let mut buf = vec![0u8; read_u32(cursor)? as usize];
    cursor.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

fn write_u32(cursor: &mut impl Write, value: u32) {
    cursor.write_varint(value).unwrap();
}

fn read_u32(cursor: &mut Cursor<Vec<u8>>) -> Option<u32> {
    ReadVarint::<u32>::read_varint(cursor).ok()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    std::fs::create_dir_all("data/downloads").unwrap();
    if !PathBuf::from_str("headers.txt").unwrap().exists() {
        println!("The `headers.txt` file is not present in the root directory.");
        println!("This file should contain your headers separated by `: `.");
        return Ok(());
    }
    let (updater_s, updater_r) = flume::unbounded::<ManagerMessage>();
    tokio::task::spawn(async {
        clean();
    });
    let updater_s = Arc::new(updater_s);
    let (sa, player) = player_system(updater_s.clone());
    downloader(sa.clone());
    {
        let updater_s = updater_s.clone();
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
