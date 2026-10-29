//! Plays a song while it is still being downloaded.
//!
//! YouTube audio (itag 140) is a fragmented mp4: the metadata sits at the
//! start of the file and the audio follows in small fragments, so it can be
//! decoded front to back while the downloader keeps appending to the file.
//!
//! - [`GrowingFile`] is a reader that waits for more bytes instead of
//!   reporting the end of the file while the download is running.
//! - [`StreamingSource`] decodes on its own thread into a bounded buffer and
//!   plays silence when the buffer runs dry, so a slow network never blocks
//!   the audio callback.

mod growing_file;
mod streaming_source;

use std::sync::Arc;

pub use growing_file::GrowingFile;
pub use streaming_source::{OpenError, StreamingSource};

/// Tells whether the downloader finished writing the file.
pub type IsComplete = Arc<dyn Fn() -> bool + Send + Sync>;
