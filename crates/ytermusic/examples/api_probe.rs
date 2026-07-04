use std::path::PathBuf;
use ytpapi2::YoutubeMusicInstance;

#[tokio::main]
async fn main() {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: cargo run -p ytermusic --example api_probe -- <headers-path>");
        std::process::exit(2);
    };
    let api = match YoutubeMusicInstance::from_header_file(&path).await {
        Ok(api) => api,
        Err(error) => {
            eprintln!("failed to initialize API: {error:?}");
            std::process::exit(1);
        }
    };
    match api.search("rock", 0).await {
        Ok(result) => println!(
            "search succeeded: {} videos, {} playlists",
            result.videos.len(),
            result.playlists.len()
        ),
        Err(error) => {
            eprintln!("search failed: {error:?}");
            std::process::exit(1);
        }
    }
}
