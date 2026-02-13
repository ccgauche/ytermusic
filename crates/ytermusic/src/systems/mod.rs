use download_manager::DownloadManager;
use once_cell::sync::Lazy;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    DATABASE,
};

pub mod logger;
pub mod player;

pub static DOWNLOAD_MANAGER: Lazy<DownloadManager> = Lazy::new(|| {
    DownloadManager::new(
        CACHE_DIR.to_path_buf(),
        &DATABASE,
        CONFIG.global.parallel_downloads,
    )
});
