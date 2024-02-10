use consts::{CACHE_DIR, HEADER_TUTORIAL};
use flume::{Receiver, Sender};
use log::error;
use once_cell::sync::Lazy;
use structures::performance::STARTUP_TIME;
use term::{Manager, ManagerMessage};
use tokio::select;

use std::{
    future::Future,
    panic,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};
use systems::{logger::init, player::player_system};

use crate::{consts::HEADER_TUTORIAL, systems::logger::get_log_file_path, utils::get_project_dirs};

mod config;
mod consts;
mod database;
mod errors;
mod structures;
mod systems;
mod term;
mod utils;

mod tasks;

pub use database::*;

use mimalloc::MiMalloc;

// Changes the allocator to improve performance especially on Windows
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub static SIGNALING_STOP: Lazy<(Sender<()>, Receiver<()>)> = Lazy::new(flume::unbounded);

fn run_service<T>(future: T) -> tokio::task::JoinHandle<()>
where
    T: Future + Send + 'static,
{
    tokio::task::spawn(async move {
        select! {
            _ = future => {},
            _ = SIGNALING_STOP.1.recv_async() => {},
        }
    })
}

fn shutdown() {
    for _ in 0..1000 {
        SIGNALING_STOP.0.send(()).unwrap();
    }
    exit(0);
}

#[tokio::main]
async fn main() {
    panic::set_hook(Box::new(|e| {
        println!("{e}");
        error!("{e}");
        shutdown();
    }));
    select! {
        _ = async {
            app_start().await
        } => {},
        _ = SIGNALING_STOP.1.recv_async() => {},
        _ = tokio::signal::ctrl_c() => {
            shutdown();
        },
    };
}
fn get_header_file() -> Result<(String, PathBuf), (std::io::Error, PathBuf)> {
    let fp = PathBuf::from_str("headers.txt").unwrap();
    if let Ok(e) = std::fs::read_to_string(&fp) {
        return Ok((e, fp));
    }
    let fp = get_project_dirs()
        .ok_or_else(|| {
            (
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Can't find project dir. This is a `directories` crate issue",
                ),
                Path::new("./").to_owned(),
            )
        })?
        .config_dir()
        .to_owned();
    if let Err(e) = std::fs::create_dir_all(&fp) {
        println!("Can't create app directory {e} in `{}`", fp.display());
    }
    let fp = fp.join("headers.txt");
    std::fs::read_to_string(&fp).map_or_else(|e| Err((e, fp.clone())), |e| Ok((e, fp.clone())))

}
async fn app_start() {
    std::fs::write(get_log_file_path(), "# YTerMusic log file\n\n").unwrap();
    init().expect("Failed to initialize logger");
    STARTUP_TIME.log("Init");

    std::fs::create_dir_all(CACHE_DIR.join("downloads")).unwrap();

    if let Err((error, filepath)) = get_header_file() {
        println!("Can't read or find `{}`", filepath.display());
        println!("Error: {error}");
        println!("{HEADER_TUTORIAL}");
        return;
    }

    STARTUP_TIME.log("Startup");

    // Spawn the clean task
    let (updater_s, updater_r) = flume::unbounded::<ManagerMessage>();
    tasks::clean::spawn_clean_task();

    STARTUP_TIME.log("Spawned clean task");
    // Spawn the player task
    let (sa, player) = player_system(updater_s.clone());
    // Spawn the downloader system
    systems::download::spawn_system(&sa);
    STARTUP_TIME.log("Spawned system task");
    tasks::last_playlist::spawn_last_playlist_task(updater_s.clone());
    STARTUP_TIME.log("Spawned last playlist task");
    // Spawn the API task
    tasks::api::spawn_api_task(updater_s.clone());
    STARTUP_TIME.log("Spawned api task");
    // Spawn the database getter task
    tasks::local_musics::spawn_local_musics_task(updater_s);

    STARTUP_TIME.log("Running manager");
    let mut manager = Manager::new(sa, player).await;
    manager.run(&updater_r).unwrap();
}
