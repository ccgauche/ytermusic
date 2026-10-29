use std::process::Stdio;

use log::error;
use tokio::process::Command;
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    DownloadManager, DownloadManagerMessage, Downloader, MessageHandler, MusicDownloadStatus,
};

#[derive(Debug)]
pub enum DownloadError {
    YtDlpFailed(String),
    IoError(std::io::Error),
    #[cfg(feature = "rusty-ytdl-backend")]
    RustyYtdl(rusty_ytdl::VideoError),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::YtDlpFailed(msg) => write!(f, "yt-dlp failed: {}", msg),
            DownloadError::IoError(e) => write!(f, "IO error: {}", e),
            #[cfg(feature = "rusty-ytdl-backend")]
            DownloadError::RustyYtdl(e) => write!(f, "rusty_ytdl error: {}", e),
        }
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(e: std::io::Error) -> Self {
        DownloadError::IoError(e)
    }
}

#[cfg(feature = "rusty-ytdl-backend")]
impl From<rusty_ytdl::VideoError> for DownloadError {
    fn from(e: rusty_ytdl::VideoError) -> Self {
        DownloadError::RustyYtdl(e)
    }
}

async fn download_with_ytdlp(
    video_id: &str,
    output_path: &std::path::Path,
    sender: &MessageHandler,
) -> Result<(), DownloadError> {
    sender(DownloadManagerMessage::VideoStatusUpdate(
        video_id.to_string(),
        MusicDownloadStatus::Downloading(0),
    ));

    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    let output = Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "-f",
            "bestaudio[ext=m4a]/bestaudio[ext=mp4]/bestaudio",
            "--merge-output-format",
            "mp4",
            "-o",
            output_path.to_str().unwrap(),
            "--no-progress",
            "--quiet",
            &url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Err(DownloadError::YtDlpFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    sender(DownloadManagerMessage::VideoStatusUpdate(
        video_id.to_string(),
        MusicDownloadStatus::Downloading(100),
    ));

    Ok(())
}

#[cfg(feature = "rusty-ytdl-backend")]
async fn download_with_rusty_ytdl(
    video_id: &str,
    output_path: &std::path::Path,
    sender: &MessageHandler,
) -> Result<(), DownloadError> {
    use rusty_ytdl::{DownloadOptions, Video, VideoOptions, VideoQuality, VideoSearchOptions};
    use std::io::Write;
    use std::sync::Arc;

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

    let video = Video::new_with_options(video_id, video_options)?;

    sender(DownloadManagerMessage::VideoStatusUpdate(
        video_id.to_string(),
        MusicDownloadStatus::Downloading(0),
    ));

    let stream = video.stream().await?;
    let length = stream.content_length();

    let mut file = std::fs::File::create(output_path)
        .map_err(|e| rusty_ytdl::VideoError::DownloadError(e.to_string()))?;

    let mut total = 0;
    while let Some(chunk) = stream.chunk().await? {
        total += chunk.len();
        sender(DownloadManagerMessage::VideoStatusUpdate(
            video_id.to_string(),
            MusicDownloadStatus::Downloading((total as f64 / length as f64 * 100.0) as usize),
        ));
        file.write_all(&chunk)
            .map_err(|e| rusty_ytdl::VideoError::DownloadError(e.to_string()))?;
    }

    file.flush()
        .map_err(|e| rusty_ytdl::VideoError::DownloadError(e.to_string()))?;

    if total != length || length == 0 {
        std::fs::remove_file(output_path)
            .map_err(|e| rusty_ytdl::VideoError::DownloadError(e.to_string()))?;
        return Err(rusty_ytdl::VideoError::DownloadError(format!(
            "Downloaded file is not the same size as the content length ({}/{})",
            total, length
        ))
        .into());
    }

    sender(DownloadManagerMessage::VideoStatusUpdate(
        video_id.to_string(),
        MusicDownloadStatus::Downloading(100),
    ));

    Ok(())
}

impl DownloadManager {
    async fn handle_download(&self, id: &str, sender: MessageHandler) -> Result<(), DownloadError> {
        let file = self.cache_dir.join("downloads").join(format!("{id}.mp4"));
        match self.downloader {
            Downloader::YtDlp => download_with_ytdlp(id, &file, &sender).await,
            #[cfg(feature = "rusty-ytdl-backend")]
            Downloader::RustyYtdl => download_with_rusty_ytdl(id, &file, &sender).await,
        }
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
            .join(format!("downloads/{}.mp4", song.video_id));
        let download_path_json = self
            .cache_dir
            .join(format!("downloads/{}.json", song.video_id));
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
                error!("couldn't download {}: {e}", song.video_id);
                false
            }
        }
    }

    pub fn start_task_unary(
        &'static self,
        s: MessageHandler,
        song: YoutubeMusicVideoRef,
        cancelation: impl Future<Output = ()> + Send + 'static,
    ) {
        let fut = async move {
            self.start_download(song, s).await;
        };
        let service = tokio::task::spawn(async move {
            tokio::select! {
                _ = fut => {},
                _ = cancelation => {},
            }
        });
        self.handles.lock().unwrap().push(service);
    }
}
