use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use atomic_float::AtomicF32;

use super::{queue, source::Done, Sample, Source};
use super::{OutputStreamHandle, PlayError};

/// Handle to an device that outputs sounds.
///
/// Dropping the `Sink` stops all sounds. You can use `detach` if you want the sounds to continue
/// playing.
pub struct Sink {
    queue_tx: Arc<queue::SourcesQueueInput<f32>>,

    controls: Arc<Controls>,
    sound_playing: Arc<AtomicBool>,

    detached: bool,

    elapsed: Arc<AtomicU32>,
}

struct Controls {
    pause: AtomicBool,
    volume: AtomicF32,
    seek: Mutex<Option<Duration>>,
    stopped: AtomicBool,
}

#[allow(unused, clippy::missing_const_for_fn)]
impl Sink {
    /// Builds a new `Sink`, beginning playback on a stream.
    #[inline]
    pub fn try_new(stream: &OutputStreamHandle) -> Result<Self, PlayError> {
        let (sink, queue_rx) = Self::new_idle();
        stream.play_raw(queue_rx)?;
        Ok(sink)
    }

    /// Builds a new `Sink`.
    #[inline]
    pub fn new_idle() -> (Self, queue::SourcesQueueOutput<f32>) {
        let (queue_tx, queue_rx) = queue::queue(true);

        let sink = Self {
            queue_tx,
            controls: Arc::new(Controls {
                pause: AtomicBool::new(false),
                volume: AtomicF32::new(1.0),
                stopped: AtomicBool::new(false),
                seek: Mutex::new(None),
            }),
            sound_playing: Arc::new(AtomicBool::new(false)),
            detached: false,
            elapsed: Arc::new(AtomicU32::new(0)),
        };
        (sink, queue_rx)
    }

    /// Appends a sound to the queue of sounds to play.
    #[inline]
    pub fn append<S>(&mut self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample + Send,
        // S::Item: Send,
    {
        let controls = self.controls.clone();

        let elapsed = self.elapsed.clone();
        let source = source
            .pausable(false)
            .amplify(1.0)
            .stoppable()
            .periodic_access(Duration::from_millis(50), move |src| {
                if controls.stopped.load(Ordering::SeqCst) {
                    src.stop();
                } else {
                    if let Some(seek_time) = controls.seek.lock().unwrap().take() {
                        match src.seek(seek_time) {
                            Ok(_) => {}
                            Err(a) => {
                                std::fs::write("error.seek", format!("Error seeking: {:?}", a));
                            }
                        }
                    }
                    elapsed.store(src.elapsed().as_secs() as u32, Ordering::Relaxed);
                    src.inner_mut().set_factor(controls.volume.load(Ordering::Relaxed));
                    src.inner_mut()
                        .inner_mut()
                        .set_paused(controls.pause.load(Ordering::Relaxed));
                }
            })
            .convert_samples::<f32>();
        self.sound_playing.store(true, Ordering::Relaxed);
        self.queue_tx.append(Done::new(source, self.sound_playing.clone()));
    }

    /// Gets the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than 1.0 will
    /// multiply each sample by this value.
    #[inline]
    pub fn volume(&self) -> f32 {
        self.controls.volume.load(Ordering::Relaxed)
    }

    /// Changes the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than `1.0` will
    /// multiply each sample by this value.
    #[inline]
    pub fn set_volume(&self, value: f32) {
        self.controls.volume.store(value, Ordering::Relaxed)
    }

    /// Resumes playback of a paused sink.
    ///
    /// No effect if not paused.
    #[inline]
    pub fn play(&self) {
        self.controls.pause.store(false, Ordering::SeqCst);
    }

    /// Pauses playback of this sink.
    ///
    /// No effect if already paused.
    ///
    /// A paused sink can be resumed with `play()`.
    pub fn pause(&self) {
        self.controls.pause.store(true, Ordering::SeqCst);
    }

    /// Toggles playback of the sink
    pub fn toggle_playback(&self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    pub fn seek(&self, seek_time: Duration) {
        *self.controls.seek.lock().unwrap() = Some(seek_time);
    }

    /// Gets if a sink is paused
    ///
    /// Sinks can be paused and resumed using `pause()` and `play()`. This returns `true` if the
    /// sink is paused.
    pub fn is_paused(&self) -> bool {
        self.controls.pause.load(Ordering::SeqCst)
    }

    /// Destroys the sink without stopping the sounds that are still playing.
    #[inline]
    pub fn detach(mut self) {
        self.detached = true;
    }

    /// Returns true if this sink has no more sounds to play.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !self.sound_playing.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn elapsed(&self) -> u32 {
        self.elapsed.load(Ordering::Relaxed)
    }
    pub fn destroy(&self) {
        self.queue_tx.set_keep_alive_if_empty(false);

        if !self.detached {
            self.controls.stopped.store(true, Ordering::Relaxed);
        }
    }
}
