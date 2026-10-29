use std::{
    fmt, io,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use flume::{Receiver, Sender, TryRecvError};
use rodio::{
    ChannelCount, Decoder, Sample, SampleRate, Source, decoder::DecoderError, source::SeekError,
};

use crate::{GrowingFile, IsComplete};

/// Give up when the download sends nothing new for this long.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Wait this long for the first decoded audio before failing to open.
const FIRST_CHUNK_TIMEOUT: Duration = Duration::from_secs(10);
const CHUNK_FRAMES: usize = 1024;
/// Decoded audio kept ahead of playback (about 1.5s of 44.1kHz audio).
const BUFFERED_CHUNKS: usize = 64;

#[derive(Debug)]
pub enum OpenError {
    Io(io::Error),
    Decoder(DecoderError),
    /// The decoder produced no audio in time.
    NoAudio,
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Io(e) => write!(f, "IO error: {e}"),
            OpenError::Decoder(e) => write!(f, "decoder error: {e}"),
            OpenError::NoAudio => write!(f, "no audio decoded in time"),
        }
    }
}

impl std::error::Error for OpenError {}

/// A rodio source playing a file that is still being downloaded.
///
/// Seeking is not supported: the decoder reads the file front to back.
pub struct StreamingSource {
    receiver: Receiver<Vec<Sample>>,
    chunk: Vec<Sample>,
    position: usize,
    /// Samples of silence left to complete the current frame on underrun.
    silence: u16,
    channels: ChannelCount,
    sample_rate: SampleRate,
    total_duration: Option<Duration>,
    cancelled: Arc<AtomicBool>,
}

impl StreamingSource {
    /// Opens `path` and decodes the first chunk of audio. Blocks until the
    /// header is readable, so call it once some data is downloaded.
    pub fn open(path: &Path, is_complete: IsComplete) -> Result<Self, OpenError> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let result = Self::start(path, is_complete, cancelled.clone());
        if result.is_err() {
            // Unblocks the decoder thread if it is waiting for data.
            cancelled.store(true, Ordering::Relaxed);
        }
        result
    }

    fn start(
        path: &Path,
        is_complete: IsComplete,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self, OpenError> {
        let file = GrowingFile::open(path, is_complete, cancelled.clone(), STALL_TIMEOUT)
            .map_err(OpenError::Io)?;
        let decoder = Decoder::new(file).map_err(OpenError::Decoder)?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let total_duration = decoder.total_duration();

        let (sender, receiver) = flume::bounded(BUFFERED_CHUNKS);
        let chunk_len = CHUNK_FRAMES * channels as usize;
        thread::Builder::new()
            .name("stream-decoder".to_string())
            .spawn(move || decode_into(decoder, sender, chunk_len))
            .map_err(OpenError::Io)?;

        let chunk = receiver
            .recv_timeout(FIRST_CHUNK_TIMEOUT)
            .map_err(|_| OpenError::NoAudio)?;

        Ok(Self {
            receiver,
            chunk,
            position: 0,
            silence: 0,
            channels,
            sample_rate,
            total_duration,
            cancelled,
        })
    }
}

/// Runs on the decoder thread until the song ends or the source is dropped.
fn decode_into(decoder: Decoder<GrowingFile>, sender: Sender<Vec<Sample>>, chunk_len: usize) {
    let mut chunk = Vec::with_capacity(chunk_len);
    for sample in decoder {
        chunk.push(sample);
        if chunk.len() == chunk_len {
            let full = std::mem::replace(&mut chunk, Vec::with_capacity(chunk_len));
            if sender.send(full).is_err() {
                return;
            }
        }
    }
    if !chunk.is_empty() {
        let _ = sender.send(chunk);
    }
}

impl Iterator for StreamingSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if let Some(sample) = self.chunk.get(self.position) {
            self.position += 1;
            return Some(*sample);
        }
        if self.silence > 0 {
            self.silence -= 1;
            return Some(0.0);
        }
        match self.receiver.try_recv() {
            Ok(chunk) => {
                self.chunk = chunk;
                self.position = 1;
                self.chunk.first().copied()
            }
            // Chunks hold whole frames, so the silence starts on a frame boundary.
            Err(TryRecvError::Empty) => {
                self.silence = self.channels - 1;
                Some(0.0)
            }
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

impl Source for StreamingSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    fn try_seek(&mut self, _: Duration) -> Result<(), SeekError> {
        Err(SeekError::NotSupported {
            underlying_source: std::any::type_name::<Self>(),
        })
    }
}

impl Drop for StreamingSource {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
