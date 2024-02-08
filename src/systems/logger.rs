use crate::utils::get_project_dirs;

use std::io::Write;

use flume::Sender;
use log::{info, Level, LevelFilter, Metadata, Record, SetLoggerError};
use once_cell::sync::Lazy;

static LOG: Lazy<Sender<String>> = Lazy::new(|| {
    let (tx, rx) = flume::unbounded::<String>();
    std::thread::spawn(move || {
        let mut buffer = String::new();

        // Open file at ~/.cache/ytermusic/log.txt (or cache alternative)
        let file_path = get_project_dirs()
            .expect("Could not find project directories!")
            .cache_dir()
            .join("log.txt");

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .unwrap();

        while let Ok(e) = rx.recv() {
            buffer.clear();
            buffer.push_str(&(e + "\n"));
            while let Ok(e) = rx.try_recv() {
                buffer.push_str(&(e + "\n"));
            }
            file.write_all(buffer.as_bytes()).unwrap();
        }
    });
    tx
});

static LOGGER: SimpleLogger = SimpleLogger;
static LEVEL: Lazy<(LevelFilter, Level)> = Lazy::new(|| {
    let logger_env = std::env::var("YTERMUSIC_LOG");
    if let Ok(logger_env) = logger_env {
        if logger_env == "true" {
            (LevelFilter::Trace, Level::Trace)
        } else {
            (LevelFilter::Info, Level::Info)
        }
    } else {
        (LevelFilter::Info, Level::Info)
    }
});

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LEVEL.0))?;
    std::fs::write(
        get_project_dirs()
            .expect("Could not find project directories!")
            .cache_dir()
            .join("log.txt"),
        "# YTerMusic Log File\n\n",
    )
    .expect("Failed to write to log file!");
    info!("Logger mode {}", LEVEL.1);
    Ok(())
}

static FILTER: &[&str] = &["rustls", "tokio-util", "want-", "mio-"];

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LEVEL.1
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Filter logs from specific crates out
            if FILTER.iter().any(|x| record.file().unwrap().contains(x)) {
                return;
            }

            LOG.send(format!(
                "{} - {} [{}]",
                record.level(),
                record.args(),
                record.file().unwrap_or_default()
            ))
            .unwrap();
        }
    }

    fn flush(&self) {}
}
