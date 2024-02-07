use std::{collections::VecDeque, sync::Mutex, time::Duration};

use flume::Sender;
use once_cell::sync::Lazy;
use tokio::{task::JoinHandle, time::sleep};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    run_service,
    structures::sound_action::SoundAction,
    tasks::download::{start_download, IN_DOWNLOAD},
};

pub static HANDLES: Lazy<Mutex<Vec<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static DOWNLOAD_LIST: Lazy<Mutex<VecDeque<YoutubeMusicVideoRef>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

fn take() -> Option<YoutubeMusicVideoRef> {
    DOWNLOAD_LIST.lock().unwrap().pop_front()
}

/// A worker of this system that downloads pending songs
fn spawn_system_worker_instance(s: Sender<SoundAction>) {
    HANDLES.lock().unwrap().push(run_service(async move {
        loop {
            if let Some(id) = take() {
                start_download(id, &s).await;
            } else {
                sleep(Duration::from_millis(200)).await;
            }
        }
    }));
}

/// Destroy all the worker and task getting processed and starts back the system
pub fn clean(sender: &Sender<SoundAction>) {
    DOWNLOAD_LIST.lock().unwrap().clear();

    IN_DOWNLOAD.lock().unwrap().clear();
    {
        let mut handle = HANDLES.lock().unwrap();
        for i in handle.iter() {
            i.abort()
        }
        handle.clear();
    }
    spawn_system(&sender);
}

/// Append a video to the download queue to be processed by the system
/* pub fn add(video: Video, s: &Sender<SoundAction>) {
    let download_path_json = CACHE_DIR.join(format!("downloads/{}.json", &video.video_id));
    if download_path_json.exists() {
        s.send(SoundAction::Up(video)).unwrap();
    } else {
        DOWNLOAD_QUEUE.lock().unwrap().push_back(video);
    }
} */

const DOWNLOADER_COUNT: usize = 4;

pub fn spawn_system(s: &Sender<SoundAction>) {
    for _ in 0..DOWNLOADER_COUNT {
        spawn_system_worker_instance(s.clone());
    }
}
