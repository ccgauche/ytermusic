use std::sync::Arc;

use flume::Sender;
use log::info;
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    consts::CACHE_DIR,
    run_service,
    structures::performance,
    term::{ManagerMessage, Screens},
};

pub fn spawn_last_playlist_task(updater_s: Sender<ManagerMessage>) {
    run_service(async move {
        let guard = performance::guard("Last playlist");
        info!("Last playlist task on");
        let playlist = std::fs::read_to_string(CACHE_DIR.join("last-playlist.json")).ok()?;
        let mut playlist: (String, Vec<YoutubeMusicVideoRef>) =
            serde_json::from_str(&playlist).ok()?;
        if !playlist.0.starts_with("Last playlist: ") {
            playlist.0 = format!("Last playlist: {}", playlist.0);
        }
        updater_s
            .send(ManagerMessage::AddElementToChooser(playlist).pass_to(Screens::Playlist))
            .unwrap();
        drop(guard);
        Some(())
    });
}
