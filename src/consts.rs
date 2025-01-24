use std::path::PathBuf;

use log::warn;
use once_cell::sync::Lazy;

use crate::{config, utils::get_project_dirs};

pub const HEADER_TUTORIAL: &str = r#"To configure the YTerMusic:
1. Open the YouTube Music website in your browser
2. Open the developer tools (F12)
3. Go to the Network tab
4. Go to https://music.youtube.com
5. Copy the `cookie` header from the associated request
6. Paste it in the `headers.txt` file in format `Cookie: <cookie>`
7. On a newline of `headers.txt` add a user-agent in format `User-Agent: <Mozilla/5.0 (Example)>
8. Restart YterMusic"#;

pub static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let pdir = get_project_dirs();
    if let Some(dir) = pdir {
        return dir.cache_dir().to_path_buf();
    };
    warn!("Failed to get cache dir! Defaulting to './data'");
    PathBuf::from("./data")
});

pub static CONFIG: Lazy<config::Config> = Lazy::new(config::Config::new);

pub const INTRODUCTION: &str = r#"Usage: ytermusic [options]

YTerMusic is a TUI based Youtube Music Player that aims to be as fast and simple as possible.
In order to get your music, create a file "headers.txt" in the config folder, and copy the Cookie and User-Agent from request header of the music.youtube.com html document "/" page.
More info at: https://github.com/ccgauche/ytermusic

Options:
        -h or --help        Show this menu
        --files             Show the location of the ytermusic files
        --fix-db            Fix the database in cache
        --clear-cache       Erase all the files in cache

Shortcuts:
        Use your mouse to click in lists if your terminal has mouse support
        Space                     play/pause
        Enter                     select a playlist or a music
        f                         search
        s                         shuffle
        Arrow Right or >          skip 5 seconds
        Arrow Left or <           go back 5 seconds
        CTRL + Arrow Right (>)    go to the next song
        CTRL + Arrow Left  (<)    go to the previous song
        +                         volume up
        -                         volume down
        Arrow down                scroll down
        Arrow up                  scroll up
        ESC                       exit the current menu
        CTRL + C or CTRL + D      quit
"#;
