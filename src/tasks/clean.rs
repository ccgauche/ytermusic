use crate::{consts::CACHE_DIR, structures::performance};

/// This function is called on start to clean the database and the files
/// that are incompletely downloaded due to a crash.
pub fn spawn_clean_task() {
    tokio::task::spawn(async move {
        let guard = performance::guard("Clean task");
        for i in std::fs::read_dir(CACHE_DIR.join("downloads")).unwrap() {
            let path = i.unwrap().path();
            if path.ends_with(".mp4") {
                let mut path1 = path.clone();
                path1.set_extension("json");
                if !path1.exists() {
                    std::fs::remove_file(&path).unwrap();
                }
            }
        }
        drop(guard);
    });
}
