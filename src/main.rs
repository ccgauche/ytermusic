use consts::CACHE_DIR;
use rustube::Error;
use term::{Manager, ManagerMessage};

use std::{path::PathBuf, str::FromStr, sync::Arc};
use systems::download::downloader;
use systems::player::player_system;

use ytpapi::Video;

use crate::consts::HEADER_TUTORIAL;
use crate::systems::logger::log_;

mod consts;
mod database;
mod errors;
mod systems;
mod term;

mod tasks;

pub use database::*;

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

#[tokio::main]
async fn main() -> Result<(), Error> {
    std::fs::write("log.txt", "# YTerMusic log file\n\n").unwrap();
    std::fs::create_dir_all(CACHE_DIR.join("downloads")).unwrap();
    if !PathBuf::from_str("headers.txt").unwrap().exists() {
        println!("The `headers.txt` file is not present in the root directory.");
        println!("{HEADER_TUTORIAL}");
        return Ok(());
    }
    if !std::fs::read_to_string("headers.txt")
        .unwrap()
        .contains("Cookie: ")
    {
        println!("The `headers.txt` file is not configured correctly.");
        println!("{HEADER_TUTORIAL}");
        return Ok(());
    }

    log_("Starting YTerMusic");

    // Spawn the clean task
    let (updater_s, updater_r) = flume::unbounded::<ManagerMessage>();
    tasks::clean::spawn_clean_task();
    let updater_s = Arc::new(updater_s);
    // Spawn the player task
    let (sa, player) = player_system(updater_s.clone());
    // Spawn the downloader task
    downloader(sa.clone());
    tasks::last_playlist::spawn_last_playlist_task(updater_s.clone());
    // Spawn the API task
    tasks::api::spawn_api_task(updater_s.clone());
    // Spawn the database getter task
    tasks::local_musics::spawn_local_musics_task(updater_s.clone());

    log_("Running the manager");
    let mut manager = Manager::new(sa, player).await;
    manager.run(&updater_r).unwrap();
    Ok(())
}
