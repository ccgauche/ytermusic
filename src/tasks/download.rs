use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use flume::Sender;
use log::error;
use once_cell::sync::Lazy;
use rustube::{Callback, Id};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    consts::CACHE_DIR,
    run_service,
    structures::{app_status::MusicDownloadStatus, sound_action::SoundAction},
    systems::{download::HANDLES},
    Error,
};

async fn handle_download(id: &str, sender: Sender<SoundAction>) -> Result<(), Error> {
    let idc = id.to_string();
    rustube::Video::from_id(Id::from_str(id)?.into_owned())
        .await?
        .streams()
        .iter()
        .filter(|stream| {
            stream.mime == "audio/mp4"
                && stream.includes_audio_track
                && !stream.includes_video_track
        })
        .max_by_key(|stream| stream.bitrate)
        .ok_or(Error::NoStreams)?
        .download_to_dir_with_callback(
            CACHE_DIR.join("downloads"),
            Callback::new().connect_on_progress_closure(move |progress| {
                let perc = progress
                    .content_length
                    .as_ref()
                    .map(|x| progress.current_chunk * 100 / *x as usize)
                    .unwrap_or(0);
                sender
                    .send(SoundAction::VideoStatusUpdate(
                        idc.clone(),
                        MusicDownloadStatus::Downloading(perc),
                    ))
                    .unwrap();
            }),
        )
        .await?;
    Ok(())
}

pub static IN_DOWNLOAD: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub async fn start_download(song: YoutubeMusicVideoRef, s: &Sender<SoundAction>) -> bool {
    {
        let mut downloads = IN_DOWNLOAD.lock().unwrap();
        if downloads.contains(&song.video_id) {
            return false;
        }
        downloads.insert(song.video_id.clone());
    }
    s.send(SoundAction::VideoStatusUpdate(
        song.video_id.clone(),
        MusicDownloadStatus::Downloading(1),
    ))
    .unwrap();
    let download_path_mp4 = CACHE_DIR.join(format!("downloads/{}.mp4", &song.video_id));
    let download_path_json = CACHE_DIR.join(format!("downloads/{}.json", &song.video_id));
    if download_path_json.exists() {
        s.send(SoundAction::VideoStatusUpdate(
            song.video_id.clone(),
            MusicDownloadStatus::Downloaded,
        ))
        .unwrap();
        return true;
    }
    if download_path_mp4.exists() {
        std::fs::remove_file(&download_path_mp4).unwrap();
    }
    match handle_download(&song.video_id, s.clone()).await {
        Ok(_) => {
            std::fs::write(download_path_json, serde_json::to_string(&song).unwrap()).unwrap();
            crate::append(song.clone());
            s.send(SoundAction::VideoStatusUpdate(
                song.video_id.clone(),
                MusicDownloadStatus::Downloaded,
            ))
            .unwrap();
            IN_DOWNLOAD.lock().unwrap().remove(&song.video_id);
            true
        }
        Err(e) => {
            if download_path_mp4.exists() {
                std::fs::remove_file(download_path_mp4).unwrap();
            }
            s.send(SoundAction::VideoStatusUpdate(
                song.video_id.clone(),
                MusicDownloadStatus::DownloadFailed,
            ))
            .unwrap();
            error!("Error downloading {}: {e}", song.video_id);
            false
        }
    }
}
pub fn start_task_unary(s: Arc<Sender<SoundAction>>, song: YoutubeMusicVideoRef) {
    HANDLES.lock().unwrap().push(run_service(async move {
        start_download(song, &s).await;
    }));
}
