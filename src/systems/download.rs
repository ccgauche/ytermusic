use std::{
    collections::VecDeque,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use flume::Sender;
use once_cell::sync::Lazy;
use tokio::{task::JoinHandle, time::sleep};
use ytpapi::Video;

use crate::{
    consts::CACHE_DIR, structures::sound_action::SoundAction, tasks::download::start_download,
};

pub static IN_DOWNLOAD: Lazy<Mutex<Vec<ytpapi::Video>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static HANDLES: Lazy<Mutex<Vec<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static DOWNLOAD_MORE: AtomicBool = AtomicBool::new(true);
// TODO Maybe switch to a channel
static DOWNLOAD_QUEUE: Lazy<Mutex<VecDeque<ytpapi::Video>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

fn take() -> Option<Video> {
    DOWNLOAD_QUEUE.lock().unwrap().pop_front()
}

/**
 * A worker of this system that downloads pending songs
 */
fn spawn_system_worker_instance(s: Arc<Sender<SoundAction>>) {
    HANDLES.lock().unwrap().push(tokio::task::spawn(async move {
        let mut k = true;
        loop {
            if !k {
                sleep(Duration::from_millis(200)).await;
            } else {
                k = false;
            }
            if !DOWNLOAD_MORE.load(std::sync::atomic::Ordering::SeqCst) {
                continue;
            }
            if let Some(id) = take() {
                k = k || start_download(id, &s).await;
            }
        }
    }));
}

/**
 * Destroy all the worker and task getting processed and starts back the system
 */
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
    DOWNLOAD_MORE.store(true, std::sync::atomic::Ordering::SeqCst);
    spawn_system(sender);
}

/**
 * Append a video to the download queue to be processed by the system
 */
pub fn add(video: Video, s: &Sender<SoundAction>) {
    let download_path_json = CACHE_DIR.join(format!("downloads/{}.json", &video.video_id));
    if download_path_json.exists() {
        s.send(SoundAction::PlayVideo(video)).unwrap();
    } else {
        DOWNLOAD_QUEUE.lock().unwrap().push_back(video);
    }
}

const DOWNLOADER_COUNT: usize = 4;

pub fn add_to_in_download(video: Video) {
    IN_DOWNLOAD.lock().unwrap().push(video);
}
pub fn remove_from_in_download(video: &Video) {
    IN_DOWNLOAD
        .lock()
        .unwrap()
        .retain(|x| x.video_id != video.video_id);
}

pub fn spawn_system(s: Arc<Sender<SoundAction>>) {
    for _ in 0..DOWNLOADER_COUNT {
        spawn_system_worker_instance(s.clone());
    }
}
