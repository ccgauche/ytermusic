#![feature(try_blocks)]

use rustube::Error;

use std::{path::PathBuf, str::FromStr, sync::Arc};
use systems::download::downloader;
use systems::player::player_system;

use ytpapi::YTApi;

mod systems;
mod terminal;

#[derive(Debug, Clone)]
pub enum SoundAction {
    PlayPause,
    Plus,
    Minus,
    Previous,
    Forward,
    Backward,
    Next,
    PlayVideo(ytpapi::Video),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    std::fs::create_dir_all("data/downloads").unwrap();
    if !PathBuf::from_str("headers.txt").unwrap().exists() {
        println!("The `headers.txt` file is not present in the root directory.");
        println!("This file should contain your headers separated by `: `.");
        return Ok(());
    }
    let (updater_s, updater_r) = flume::unbounded();
    tokio::task::spawn(async {
        clean();
    });
    let sa = Arc::new(player_system(updater_s));
    let sender = downloader(sa.clone());
    let api = YTApi::from_header_file(PathBuf::from_str("headers.txt").unwrap().as_path())
        .await
        .unwrap();
    for video in api
        .browse_playlist(&api.playlists().first().unwrap().browse_id)
        .await
        .unwrap()
    {
        sender.send(video).unwrap();
    }
    terminal::main(updater_r, sa).unwrap();
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
