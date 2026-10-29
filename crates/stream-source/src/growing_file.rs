use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::IsComplete;

const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// A file that another process is still writing to.
///
/// Reads past the written data wait until more data arrives, the download
/// completes, the reader is cancelled or no data arrives for `stall_timeout`.
pub struct GrowingFile {
    file: File,
    is_complete: IsComplete,
    cancelled: Arc<AtomicBool>,
    stall_timeout: Duration,
}

impl GrowingFile {
    pub fn open(
        path: &Path,
        is_complete: IsComplete,
        cancelled: Arc<AtomicBool>,
        stall_timeout: Duration,
    ) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
            is_complete,
            cancelled,
            stall_timeout,
        })
    }
}

impl Read for GrowingFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let started = Instant::now();
        loop {
            let read = self.file.read(buf)?;
            if read > 0 || buf.is_empty() || self.cancelled.load(Ordering::Relaxed) {
                return Ok(read);
            }
            if (self.is_complete)() {
                // The last bytes may have landed between the read and the check.
                return self.file.read(buf);
            }
            if started.elapsed() >= self.stall_timeout {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "no new data from the download",
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Seek for GrowingFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::OpenOptions, io::Write};

    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("stream-source-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn waits_for_appended_data_until_complete() {
        let path = temp_path("append");
        std::fs::write(&path, b"abc").unwrap();
        let complete = Arc::new(AtomicBool::new(false));

        let writer = {
            let (path, complete) = (path.clone(), complete.clone());
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(250));
                let mut file = OpenOptions::new().append(true).open(&path).unwrap();
                file.write_all(b"def").unwrap();
                complete.store(true, Ordering::SeqCst);
            })
        };

        let is_complete: IsComplete = Arc::new(move || complete.load(Ordering::SeqCst));
        let mut reader = GrowingFile::open(
            &path,
            is_complete,
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(5),
        )
        .unwrap();
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        writer.join().unwrap();

        assert_eq!(out, b"abcdef");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn times_out_when_the_download_stalls() {
        let path = temp_path("stall");
        std::fs::write(&path, b"abc").unwrap();
        let mut reader = GrowingFile::open(
            &path,
            Arc::new(|| false),
            Arc::new(AtomicBool::new(false)),
            Duration::from_millis(200),
        )
        .unwrap();
        let mut out = Vec::new();
        let err = reader.read_to_end(&mut out).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert_eq!(out, b"abc");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stops_waiting_when_cancelled() {
        let path = temp_path("cancel");
        std::fs::write(&path, b"abc").unwrap();
        let mut reader = GrowingFile::open(
            &path,
            Arc::new(|| false),
            Arc::new(AtomicBool::new(true)),
            Duration::from_secs(60),
        )
        .unwrap();
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();

        assert_eq!(out, b"abc");
        std::fs::remove_file(path).unwrap();
    }
}
