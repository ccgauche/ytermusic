use std::sync::Arc;

use flume::Sender;
use ytpapi::Video;

use crate::{
    consts::CACHE_DIR,
    systems::logger::log_,
    term::{ManagerMessage, Screens},
};

pub fn spawn_last_playlist_task(updater_s: Arc<Sender<ManagerMessage>>) {
    tokio::task::spawn(async move {
        log_("Last playlist task on");
        let playlist = std::fs::read_to_string(CACHE_DIR.join("last-playlist.json")).ok()?;
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
