use std::path::PathBuf;
use std::str::FromStr;

use ytpapi::YTApi;

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let api = YTApi::from_header_file(PathBuf::from_str("headers.txt").unwrap().as_path())
                .await
                .unwrap();

            println!(
                "{:?}",
                api.browse_playlist("OLAK5uy_mHWxtaESBpg2TyQJW9cyhxQGaCzN5pSkg")
                    .await
            )
        });
}
