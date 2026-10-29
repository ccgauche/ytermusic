use download_manager::{DownloadManager, Downloader};
use once_cell::sync::Lazy;

use crate::{
    config::{DownloaderConfig, DEFAULT_PARALLEL_DOWNLOADS},
    consts::{CACHE_DIR, CONFIG},
    DATABASE,
};

pub mod logger;
pub mod player;

pub static DOWNLOAD_MANAGER: Lazy<DownloadManager> = Lazy::new(|| {
    let downloader = match CONFIG.global.downloader {
        DownloaderConfig::Ytdlp => Downloader::YtDlp,
        #[cfg(feature = "rusty-ytdl-backend")]
        DownloaderConfig::RustyYtdl => Downloader::RustyYtdl,
        #[cfg(not(feature = "rusty-ytdl-backend"))]
        DownloaderConfig::RustyYtdl => {
            log::warn!("rusty-ytdl-backend not compiled, using yt-dlp");
            Downloader::YtDlp
        }
    };
    // Older versions wrote `parallel_downloads = 0` into new config files,
    // which leaves no download worker and playlists never start playing
    let parallel_downloads = match CONFIG.global.parallel_downloads {
        0 => {
            log::warn!(
                "parallel_downloads = 0 would disable downloads, using {DEFAULT_PARALLEL_DOWNLOADS} instead"
            );
            DEFAULT_PARALLEL_DOWNLOADS
        }
        n => n,
    };
    DownloadManager::new(
        CACHE_DIR.to_path_buf(),
        &DATABASE,
        parallel_downloads,
        downloader,
    )
});
