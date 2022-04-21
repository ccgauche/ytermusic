use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use flume::Sender;
use once_cell::sync::Lazy;
use rustube::{Error, Id};
use tokio::{task::JoinHandle, time::sleep};
use ytpapi::Video;

use crate::SoundAction;

pub static IN_DOWNLOAD: Lazy<Mutex<Vec<ytpapi::Video>>> = Lazy::new(|| Mutex::new(Vec::new()));
static HANDLES: Lazy<Mutex<Vec<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static DOWNLOAD_MORE: AtomicBool = AtomicBool::new(true);
static DOWNLOAD_QUEUE: Lazy<Mutex<VecDeque<ytpapi::Video>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

fn take() -> Option<Video> {
    DOWNLOAD_QUEUE.lock().unwrap().pop_front()
}

pub fn clean(sender: Arc<Sender<SoundAction>>) {
    DOWNLOAD_QUEUE.lock().unwrap().clear();
    {
        let mut handle = HANDLES.lock().unwrap();
        for i in handle.iter() {
            i.abort()
        }
        handle.clear();
    }
    IN_DOWNLOAD.lock().unwrap().clear();
    downloader(sender);
}

pub fn add(video: Video) {
    DOWNLOAD_QUEUE.lock().unwrap().push_back(video);
}

async fn handle_download(id: &str) -> Result<PathBuf, Error> {
    rustube::Video::from_id(Id::from_str(id)?.into_owned())
        .await?
        .best_audio()
        .ok_or(Error::NoStreams)?
        .download(Path::new("data/downloads"))
        .await
}

const DOWNLOADER_COUNT: usize = 4;

pub fn start_task(s: Arc<Sender<SoundAction>>) {
    HANDLES.lock().unwrap().push(tokio::task::spawn(async move {
        loop {
            sleep(Duration::from_millis(200)).await;
            if !DOWNLOAD_MORE.load(std::sync::atomic::Ordering::SeqCst) {
                continue;
            }
            if let Some(id) = take() {
                // TODO(#1): handle errors
                let download_path_mp4 =
                    PathBuf::from_str(&format!("data/downloads/{}.mp4", &id.video_id)).unwrap();
                let download_path_json =
                    PathBuf::from_str(&format!("data/downloads/{}.json", &id.video_id)).unwrap();
                if download_path_json.exists() {
                    s.send(SoundAction::PlayVideo(id)).unwrap();
                    continue;
                }
                if download_path_mp4.exists() {
                    std::fs::remove_file(&download_path_mp4).unwrap();
                }
                {
                    IN_DOWNLOAD.lock().unwrap().push(id.clone());
                }
                match handle_download(&id.video_id).await {
                    Ok(_) => {
                        std::fs::write(download_path_json, serde_json::to_string(&id).unwrap())
                            .unwrap();

                        {
                            IN_DOWNLOAD
                                .lock()
                                .unwrap()
                                .retain(|x| x.video_id != id.video_id);
                        }
                        s.send(SoundAction::PlayVideo(id)).unwrap();
                    }
                    Err(_) => {
                        if download_path_mp4.exists() {
                            std::fs::remove_file(download_path_mp4).unwrap();
                        }

                        {
                            IN_DOWNLOAD
                                .lock()
                                .unwrap()
                                .retain(|x| x.video_id != id.video_id);
                        }
                        // TODO(#1): handle errors
                    }
                }
            }
        }
    }));
}

pub fn downloader(s: Arc<Sender<SoundAction>>) {
    for _ in 0..DOWNLOADER_COUNT {
        start_task(s.clone());
    }
}
