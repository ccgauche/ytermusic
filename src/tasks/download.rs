use std::sync::Arc;

use flume::Sender;
use rustube::{Callback, Id};
use ytpapi::Video;

use crate::{
    consts::CACHE_DIR,
    structures::sound_action::SoundAction,
    systems::{
        download::{add_to_in_download, remove_from_in_download, update_in_download, HANDLES},
        logger::log_,
    },
    Error,
};

async fn handle_download(id: &str) -> Result<(), Error> {
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
                update_in_download(
                    &idc,
                    progress
                        .content_length
                        .as_ref()
                        .map(|x| progress.current_chunk * 100 / *x as usize)
                        .unwrap_or(0),
                );
            }),
        )
        .await?;
    Ok(())
}

pub async fn start_download(song: Video, s: &Sender<SoundAction>) -> bool {
    let download_path_mp4 = CACHE_DIR.join(format!("downloads/{}.mp4", &song.video_id));
    let download_path_json = CACHE_DIR.join(format!("downloads/{}.json", &song.video_id));
    if download_path_json.exists() {
        s.send(SoundAction::PlayVideoUnary(song.clone())).unwrap();
        return true;
    }
    if download_path_mp4.exists() {
        std::fs::remove_file(&download_path_mp4).unwrap();
    }
    add_to_in_download(song.clone());
    match handle_download(&song.video_id).await {
        Ok(_) => {
            std::fs::write(download_path_json, serde_json::to_string(&song).unwrap()).unwrap();
            crate::append(song.clone());
            remove_from_in_download(&song);
            s.send(SoundAction::PlayVideoUnary(song)).unwrap();
            true
        }
        Err(e) => {
            if download_path_mp4.exists() {
                std::fs::remove_file(download_path_mp4).unwrap();
            }

            remove_from_in_download(&song);
            log_(format!("Error downloading {}: {e}", song.video_id));
            false
        }
    }
}
pub fn start_task_unary(s: Arc<Sender<SoundAction>>, song: Video) {
    HANDLES.lock().unwrap().push(tokio::task::spawn(async move {
        start_download(song, &s).await;
    }));
}
