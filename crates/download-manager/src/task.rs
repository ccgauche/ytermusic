use std::sync::Arc;

use flume::Receiver;
use log::error;
use rusty_ytdl::{
    DownloadOptions, Video, VideoError, VideoOptions, VideoQuality, VideoSearchOptions,
};
use tokio::select;
use ytpapi2::YoutubeMusicVideoRef;

use crate::{DownloadManager, DownloadManagerMessage, MessageHandler, MusicDownloadStatus};

fn new_video_with_id(id: &str) -> Result<Video<'_>, VideoError> {
    let search_options = VideoSearchOptions::Custom(Arc::new(|format| {
        format.has_audio && !format.has_video && format.mime_type.container == "mp4"
    }));
    let video_options = VideoOptions {
        quality: VideoQuality::Custom(
            search_options.clone(),
            Arc::new(|x, y| x.audio_bitrate.cmp(&y.audio_bitrate)),
        ),
        filter: search_options,
        download_options: DownloadOptions {
            dl_chunk_size: Some(1024 * 100_u64),
        },
        ..Default::default()
    };

    Video::new_with_options(id, video_options)
}

pub async fn download<P: AsRef<std::path::Path>>(
    video: &Video<'_>,
    path: P,
    sender: MessageHandler,
) -> Result<(), VideoError> {
    use std::io::Write;
    let stream = video.stream().await?;

    let length = stream.content_length();

    let mut file =
        std::fs::File::create(&path).map_err(|e| VideoError::DownloadError(e.to_string()))?;

    let mut total = 0;
    while let Some(chunk) = stream.chunk().await? {
        total += chunk.len();

        sender(DownloadManagerMessage::VideoStatusUpdate(
            video.get_video_id(),
            MusicDownloadStatus::Downloading((total as f64 / length as f64 * 100.0) as usize),
        ));

        file.write_all(&chunk)
            .map_err(|e| VideoError::DownloadError(e.to_string()))?;
    }

    file.flush()
        .map_err(|e| VideoError::DownloadError(e.to_string()))?;

    if total != length || length == 0 {
        std::fs::remove_file(path).map_err(|e| VideoError::DownloadError(e.to_string()))?;
        return Err(VideoError::DownloadError(format!(
            "Downloaded file is not the same size as the content length ({}/{})",
            total, length
        )));
    }

    Ok(())
}

impl DownloadManager {
    async fn handle_download(&self, id: &str, sender: MessageHandler) -> Result<(), VideoError> {
        let idc = id.to_string();

        let video = new_video_with_id(id)?;

        sender(DownloadManagerMessage::VideoStatusUpdate(
            idc.clone(),
            MusicDownloadStatus::Downloading(0),
        ));
        let file = self.cache_dir.join("downloads").join(format!("{id}.mp4"));
        download(&video, file, sender.clone()).await?;
        sender(DownloadManagerMessage::VideoStatusUpdate(
            idc.clone(),
            MusicDownloadStatus::Downloading(100),
        ));
        Ok(())
    }
    pub async fn start_download(&self, song: YoutubeMusicVideoRef, s: MessageHandler) -> bool {
        {
            let mut downloads = self.in_download.lock().unwrap();
            if downloads.contains(&song.video_id) {
                return false;
            }
            downloads.insert(song.video_id.clone());
        }
        s(DownloadManagerMessage::VideoStatusUpdate(
            song.video_id.clone(),
            MusicDownloadStatus::Downloading(1),
        ));
        let download_path_mp4 = self
            .cache_dir
            .join(format!("downloads/{}.mp4", &song.video_id));
        let download_path_json = self
            .cache_dir
            .join(format!("downloads/{}.json", &song.video_id));
        if download_path_json.exists() {
            s(DownloadManagerMessage::VideoStatusUpdate(
                song.video_id.clone(),
                MusicDownloadStatus::Downloaded,
            ));
            return true;
        }
        if download_path_mp4.exists() {
            std::fs::remove_file(&download_path_mp4).unwrap();
        }
        match self.handle_download(&song.video_id, s.clone()).await {
            Ok(_) => {
                std::fs::write(download_path_json, serde_json::to_string(&song).unwrap()).unwrap();
                self.database.append(song.clone());
                s(DownloadManagerMessage::VideoStatusUpdate(
                    song.video_id.clone(),
                    MusicDownloadStatus::Downloaded,
                ));
                self.in_download.lock().unwrap().remove(&song.video_id);
                true
            }
            Err(e) => {
                if download_path_mp4.exists() {
                    std::fs::remove_file(download_path_mp4).unwrap();
                }
                s(DownloadManagerMessage::VideoStatusUpdate(
                    song.video_id.clone(),
                    MusicDownloadStatus::DownloadFailed,
                ));
                error!("Error downloading {}: {e}", song.video_id);
                false
            }
        }
    }

    pub fn start_task_unary(
        &'static self,
        s: MessageHandler,
        song: YoutubeMusicVideoRef,
        cancelation: Receiver<()>,
    ) {
        let fut = async move {
            self.start_download(song, s).await;
        };
        let service = tokio::task::spawn(async move {
            select! {
                _ = fut => {},
                _ = cancelation.recv_async() => {},
            }
        });
        self.handles.lock().unwrap().push(service);
    }
}

#[tokio::test]
async fn video_download_test() {
    let ids = vec!["iFbNzVFgjCk", "ni-xbEK271I"]; //second not working, need checking
    for id in ids {
        let video = Video::new(id).unwrap();
        let stream = video.stream().await.unwrap();
        let content_length = stream.content_length();
        let mut total = 0;
        while let Some(chunk) = stream.chunk().await.unwrap() {
            total += chunk.len();
        }
        assert_eq!(total, content_length);
    }
}
