mod task;

use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    pin::pin,
    sync::{Arc, Mutex},
};

use database::YTLocalDatabase;
use tokio::{select, sync::Notify, task::JoinHandle};
use ytpapi2::YoutubeMusicVideoRef;

use common_structs::MusicDownloadStatus;

pub type MessageHandler = Arc<dyn Fn(DownloadManagerMessage) + Send + Sync + 'static>;

pub enum DownloadManagerMessage {
    VideoStatusUpdate(String, MusicDownloadStatus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Downloader {
    YtDlp,
    #[cfg(feature = "rusty-ytdl-backend")]
    RustyYtdl,
}

#[derive(Default)]
struct DownloadQueue {
    list: VecDeque<YoutubeMusicVideoRef>,
    /// The head of `list` is the song being played: it is downloaded alone so
    /// it gets the whole bandwidth and becomes playable as soon as possible.
    priority_pending: bool,
    /// The priority song in download. Other workers wait until it is done.
    exclusive: Option<String>,
}

pub struct DownloadManager {
    database: &'static YTLocalDatabase,
    cache_dir: PathBuf,
    parallel_downloads: u16,
    pub downloader: Downloader,
    handles: Mutex<Vec<JoinHandle<()>>>,
    queue: Mutex<DownloadQueue>,
    in_download: Mutex<HashSet<String>>,
    notify: Notify,
}

impl DownloadManager {
    pub fn new(
        cache_dir: PathBuf,
        database: &'static YTLocalDatabase,
        parallel_downloads: u16,
        downloader: Downloader,
    ) -> Self {
        Self {
            database,
            cache_dir,
            parallel_downloads,
            downloader,
            handles: Mutex::new(Vec::new()),
            queue: Mutex::new(DownloadQueue::default()),
            in_download: Mutex::new(HashSet::new()),
            notify: Notify::new(),
        }
    }

    pub fn remove_from_in_downloads(&self, video: &String) {
        self.in_download.lock().unwrap().remove(video);
    }

    fn take(&self) -> Option<YoutubeMusicVideoRef> {
        let mut queue = self.queue.lock().unwrap();
        if queue.priority_pending {
            queue.priority_pending = false;
            let song = queue.list.pop_front()?;
            queue.exclusive = Some(song.video_id.clone());
            return Some(song);
        }
        if queue.exclusive.is_some() {
            return None;
        }
        queue.list.pop_front()
    }

    /// Lets the other workers resume once the priority song is done.
    fn finish(&self, video_id: &str) {
        let mut queue = self.queue.lock().unwrap();
        if queue.exclusive.as_deref() == Some(video_id) {
            queue.exclusive = None;
            drop(queue);
            self.notify.notify_waiters();
        }
    }

    /// This has to be called as a service stream
    /// HANDLES.lock().unwrap().push(run_service(async move {
    ///     run_service_stream(sender);
    /// }));
    pub fn run_service_stream(
        &'static self,
        cancelation: impl Future<Output = ()> + Clone + Send + 'static,
        sender: MessageHandler,
    ) {
        let fut = async move {
            loop {
                // Registered before `take` so a wake-up between the two is not lost.
                let mut notified = pin!(self.notify.notified());
                notified.as_mut().enable();
                if let Some(song) = self.take() {
                    let video_id = song.video_id.clone();
                    self.start_download(song, sender.clone()).await;
                    self.finish(&video_id);
                } else {
                    notified.await;
                }
            }
        };
        let service = tokio::task::spawn(async move {
            select! {
                _ = fut => {},
                _ = cancelation => {},
            }
        });
        self.handles.lock().unwrap().push(service);
    }

    pub fn spawn_system(
        &'static self,
        cancelation: impl Future<Output = ()> + Clone + Send + 'static,
        sender: MessageHandler,
    ) {
        for _ in 0..self.parallel_downloads {
            self.run_service_stream(cancelation.clone(), sender.clone());
        }
    }

    pub fn clean(
        &'static self,
        cancelation: impl Future<Output = ()> + Clone + Send + 'static,
        sender: MessageHandler,
    ) {
        *self.queue.lock().unwrap() = DownloadQueue::default();
        self.in_download.lock().unwrap().clear();
        {
            let mut handle = self.handles.lock().unwrap();
            for i in handle.iter() {
                i.abort()
            }
            handle.clear();
        }
        self.spawn_system(cancelation, sender);
    }

    /// Replaces the download queue. When `priority_first` is set, the first song
    /// is downloaded alone before the other workers start on the rest.
    pub fn set_download_list(
        &self,
        to_add: impl IntoIterator<Item = YoutubeMusicVideoRef>,
        priority_first: bool,
    ) {
        let mut queue = self.queue.lock().unwrap();
        queue.list.clear();
        queue.list.extend(to_add);
        queue.priority_pending = priority_first;
        drop(queue);
        self.notify.notify_waiters();
    }

    pub fn add_to_download_list(&self, to_add: impl IntoIterator<Item = YoutubeMusicVideoRef>) {
        self.queue.lock().unwrap().list.extend(to_add);
        self.notify.notify_waiters();
    }
}
