use std::path::PathBuf;

use log::warn;
use once_cell::sync::Lazy;

use crate::{config, utils::get_project_dirs};

pub const HEADER_TUTORIAL: &str = r#"To configure the YTerMusic:
1. Open the YouTube Music website in your browser;
2. Open the developer tools (F12);
3. Go to the Network tab;
4. Go to https://music.youtube.com;
5. Copy the `cookie` header from the associated request;
6. Paste it in the `headers.txt` file as `Cookie: <cookie>`;
7. Restart YterMusic"#;

pub static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let pdir = get_project_dirs();
    if let Some(dir) = pdir {
        return dir.cache_dir().to_path_buf();
    };
    warn!("Failed to get cache dir! Defaulting to './data'");
    PathBuf::from("./data")
});

pub static CONFIG: Lazy<config::Config> = Lazy::new(config::Config::new);
