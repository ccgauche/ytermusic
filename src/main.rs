use consts::{CACHE_DIR, HEADER_TUTORIAL};
use flume::{Receiver, Sender};
use log::{error};
use once_cell::sync::Lazy;
use rustube::Error;
use structures::performance::STARTUP_TIME;
use term::{Manager, ManagerMessage};
use tokio::select;

use std::{future::Future, panic, str::FromStr, sync::Arc, process::exit};
use systems::{player::player_system, logger::init};

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
            app_start().await.unwrap()
        } => {},
        _ = SIGNALING_STOP.1.recv_async() => {},
        _ = tokio::signal::ctrl_c() => {
            shutdown();
        },
    };
}
async fn app_start() -> Result<(), Error> {
    std::fs::write("log.txt", "# YTerMusic log file\n\n").unwrap();
    init().expect("Failed to initialize logger");
    STARTUP_TIME.log("Init");

    std::fs::create_dir_all(CACHE_DIR.join("downloads")).unwrap();
    if let Some(headers_path) = utils::locate_headers_file() {
        //println!("Reading headers from {:?}", headers_path);
        let headers = std::fs::read_to_string(headers_path).unwrap();
        if !headers.to_lowercase().contains("cookie: ") {
            println!("The `headers.txt` file is not configured correctly.");
            println!("{HEADER_TUTORIAL}");
            return Ok(());
        }
    } else {
        println!("The `headers.txt` file was not found.");
        println!("{HEADER_TUTORIAL}");
        return Ok(());
    }

    STARTUP_TIME.log("Startup");

    // Spawn the clean task
    let (updater_s, updater_r) = flume::unbounded::<ManagerMessage>();
    tasks::clean::spawn_clean_task();

    STARTUP_TIME.log("Spawned clean task");
    let updater_s = Arc::new(updater_s);
    // Spawn the player task
    let (sa, player) = player_system(updater_s.clone());
    // Spawn the downloader system
    systems::download::spawn_system(sa.clone());
    STARTUP_TIME.log("Spawned system task");
    tasks::last_playlist::spawn_last_playlist_task(updater_s.clone());
    STARTUP_TIME.log("Spawned last playlist task");
    // Spawn the API task
    tasks::api::spawn_api_task(updater_s.clone());
    STARTUP_TIME.log("Spawned api task");
    // Spawn the database getter task
    tasks::local_musics::spawn_local_musics_task(updater_s.clone());

    STARTUP_TIME.log("Running manager");
    let mut manager = Manager::new(sa, player).await;
    manager.run(&updater_r).unwrap();
    Ok(())
}
