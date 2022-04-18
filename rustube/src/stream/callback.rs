use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use futures::FutureExt;
use tokio::sync::{mpsc::{Receiver, Sender}, Mutex};
use tokio::sync::mpsc;

use crate::Result;

pub type OnProgressClosure = Box<dyn Fn(CallbackArguments)>;
pub type OnProgressAsyncClosure = Box<dyn Fn(CallbackArguments) -> Pin<Box<dyn Future<Output=()>>>>;
pub type OnCompleteClosure = Box<dyn Fn(Option<PathBuf>)>;
pub type OnCompleteAsyncClosure = Box<dyn Fn(Option<PathBuf>) -> Pin<Box<dyn Future<Output=()>>>>;

#[derive(Debug)]
pub(crate) enum InternalSignal {
    Value(usize),
    Finished,
}

pub(crate) type InternalSender = Sender<InternalSignal>;

/// Arguments given either to a on_progress callback or on_progress receiver
#[doc(cfg(feature = "callback"))]
#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct CallbackArguments {
    pub current_chunk: usize,
    /// It's more idiomatic to use this content length instead of a prefetched value
    /// since the content of this field might change in the future during the download.
    pub content_length: Option<u64>,
}

/// Type to process on_progress
#[doc(cfg(feature = "callback"))]
pub enum OnProgressType {
    /// Box containing a closure to execute on progress
    Closure(OnProgressClosure),
    /// Box containing a async closure to execute on progress
    AsyncClosure(OnProgressAsyncClosure),
    /// Channel to send a message to on progress,
    /// bool indicates whether or not to cancel on a closed channel
    Channel(Sender<CallbackArguments>, bool),
    /// Box containing a closure to execute on progress
    /// Will get executed for every MB downloaded
    SlowClosure(OnProgressClosure),
    /// Box containing a async closure to execute on progress
    /// Will get executed for every MB downloaded
    SlowAsyncClosure(OnProgressAsyncClosure),
    /// Channel to send a message to on progress,
    /// bool indicates whether or not to cancel on a closed channel
    /// Will get executed for every MB downloaded
    SlowChannel(Sender<CallbackArguments>, bool),
    None,
}

impl fmt::Debug for OnProgressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            OnProgressType::AsyncClosure(_) => "AsyncClosure(async Fn)",
            OnProgressType::Channel(_, _) => "Channel(Sender, bool)",
            OnProgressType::Closure(_) => "Closure(Fn)",
            OnProgressType::None => "None",
            OnProgressType::SlowAsyncClosure(_) => "SlowAsyncClosure(async Fn)",
            OnProgressType::SlowChannel(_, _) => "SlowChannel(Sender, bool)",
            OnProgressType::SlowClosure(_) => "SlowClosure(Fn)",
        };
        f.write_str(name)
    }
}

#[doc(cfg(feature = "callback"))]
impl Default for OnProgressType {
    fn default() -> Self {
        OnProgressType::None
    }
}

/// Type to process on_progress
#[doc(cfg(feature = "callback"))]
pub enum OnCompleteType {
    /// Box containing a closure to execute on complete
    Closure(OnCompleteClosure),
    /// Box containing a async closure to execute on complete
    AsyncClosure(OnCompleteAsyncClosure),
    None,
}

impl fmt::Debug for OnCompleteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            OnCompleteType::AsyncClosure(_) => "AsyncClosure(async Fn)",
            OnCompleteType::Closure(_) => "Closure(Fn)",
            OnCompleteType::None => "None",
        };
        f.write_str(name)
    }
}

#[doc(cfg(feature = "callback"))]
impl Default for OnCompleteType {
    fn default() -> Self {
        OnCompleteType::None
    }
}

/// Methods and streams to process either on_progress or on_complete
#[doc(cfg(feature = "callback"))]
#[derive(Debug)]
pub struct Callback {
    pub on_progress: OnProgressType,
    pub on_complete: OnCompleteType,
    pub(crate) internal_sender: InternalSender,
    pub(crate) internal_receiver: Option<Receiver<InternalSignal>>,
}

#[doc(cfg(feature = "callback"))]
impl Callback {
    /// Create a new callback struct without actual callbacks
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Callback {
            on_progress: OnProgressType::None,
            on_complete: OnCompleteType::None,
            internal_sender: tx,
            internal_receiver: Some(rx),
        }
    }

    /// Attach a closure to be executed on progress
    ///
    /// ### Warning:
    /// This closure gets executed quite often, once every ~10kB progress.
    /// If it's too slow, some on_progress events will be dropped.
    /// If you are looking fore something that will be executed more seldom, look for
    /// [Callback::connect_on_progress_closure_slow](crate::stream::callback::Callback::connect_on_progress_closure_slow)
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_progress_closure(mut self, closure: impl Fn(CallbackArguments) + 'static) -> Self {
        self.on_progress = OnProgressType::Closure(Box::new(closure));
        self
    }

    /// Attach a closure to be executed on progress. This closure will be executed
    /// more seldom, around once for every MB downloaded.
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_progress_closure_slow(mut self, closure: impl Fn(CallbackArguments) + 'static) -> Self {
        self.on_progress = OnProgressType::SlowClosure(Box::new(closure));
        self
    }

    /// Attach a async closure to be executed on progress
    ///
    /// ### Warning:
    /// This closure gets executed quite often, once every ~10kB progress.
    /// If it's too slow, some on_progress events will be dropped.
    /// If you are looking fore something that will be executed more seldom, look for
    /// [Callback::connect_on_progress_closure_async_slow](crate::stream::callback::Callback::connect_on_progress_closure_async_slow)
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_progress_closure_async<Fut: Future<Output=()> + Send + 'static, F: Fn(CallbackArguments) -> Fut + 'static>(mut self, closure: F) -> Self {
        self.on_progress = OnProgressType::AsyncClosure(box move |arg| closure(arg).boxed());
        self
    }

    /// Attach a async closure to be executed on progress. This closure will be executed
    /// more seldom, around once for every MB downloaded.
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_progress_closure_async_slow<Fut: Future<Output=()> + Send + 'static, F: Fn(CallbackArguments) -> Fut + 'static + Sync + Send>(mut self, closure: F) -> Self {
        self.on_progress = OnProgressType::SlowAsyncClosure(box move |arg| closure(arg).boxed());
        self
    }

    /// Attach a bounded sender that receives messages on progress
    /// cancel_or_close indicates whether or not to cancel the download, if the receiver is closed
    ///
    /// ### Warning:
    /// This sender gets messages quite often, once every ~10kB progress.
    /// If it's too slow, some on_progress events will be dropped.
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_progress_sender(
        mut self,
        sender: Sender<CallbackArguments>,
        cancel_on_close: bool,
    ) -> Self {
        self.on_progress = OnProgressType::Channel(sender, cancel_on_close);
        self
    }

    /// Attach a bounded sender that receives messages on progress
    /// cancel_or_close indicates whether or not to cancel the download, if the receiver is closed
    ///
    /// This closure will be executed more seldom, around once for every MB downloaded.
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_progress_sender_slow(
        mut self,
        sender: Sender<CallbackArguments>,
        cancel_on_close: bool,
    ) -> Self {
        self.on_progress = OnProgressType::SlowChannel(sender, cancel_on_close);
        self
    }

    /// Attach a closure to be executed on complete
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_complete_closure(mut self, closure: impl Fn(Option<PathBuf>) + 'static) -> Self {
        self.on_complete = OnCompleteType::Closure(Box::new(closure));
        self
    }

    /// Attach a async closure to be executed on complete
    #[doc(cfg(feature = "callback"))]
    #[inline]
    #[must_use]
    pub fn connect_on_complete_closure_async<Fut: Future<Output=()> + Send + 'static, F: Fn(Option<PathBuf>) -> Fut + 'static>(mut self, closure: F) -> Self {
        self.on_complete = OnCompleteType::AsyncClosure(box move |arg| closure(arg).boxed());
        self
    }
}

impl Default for Callback {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Stream {
    /// Attempts to downloads the [`Stream`](super::Stream)s resource.
    /// This will download the video to <video_id>.mp4 in the current working directory.
    /// Takes an [`Callback`](crate::stream::callback::Callback)
    #[doc(cfg(feature = "callback"))]
    #[inline]
    pub async fn download_with_callback(&self, callback: Callback) -> Result<PathBuf> {
        self.wrap_callback(|channel| {
            self.internal_download(channel)
        }, callback).await
    }

    /// Attempts to downloads the [`Stream`](super::Stream)s resource.
    /// This will download the video to <video_id>.mp4 in the provided directory.
    /// Takes an [`Callback`](crate::stream::callback::Callback)
    #[doc(cfg(feature = "callback"))]
    #[inline]
    pub async fn download_to_dir_with_callback<P: AsRef<Path>>(
        &self,
        dir: P,
        callback: Callback,
    ) -> Result<PathBuf> {
        self.wrap_callback(|channel| {
            self.internal_download_to_dir(dir, channel)
        }, callback).await
    }

    /// Attempts to downloads the [`Stream`](super::Stream)s resource.
    /// This will download the video to the provided file path.
    /// Takes an [`Callback`](crate::stream::callback::Callback)
    #[doc(cfg(feature = "callback"))]
    #[inline]
    pub async fn download_to_with_callback<P: AsRef<Path>>(&self, path: P, callback: Callback) -> Result<()> {
        let _ = self.wrap_callback(|channel| {
            self.internal_download_to(path, channel)
        }, callback).await?;
        Ok(())
    }

    async fn wrap_callback<F: Future<Output = Result<PathBuf>>>(
        &self,
        to_wrap: impl FnOnce(Option<InternalSender>)-> F,
        mut callback: Callback
    ) -> Result<PathBuf> {
        let wrap_fut = to_wrap(Some(callback.internal_sender.clone()));
        let aid_fut = self.on_progress(
            callback.internal_receiver.take().expect("Callback cannot be used twice"),
            std::mem::take(&mut callback.on_progress),
        );
        let (result, _) = futures::future::join(wrap_fut, aid_fut).await;

        let path = result.as_ref().map(|p| p.clone()).ok();

        Self::on_complete(std::mem::take(&mut callback.on_complete), path).await;

        result
    }

    #[inline]
    async fn on_progress(&self, mut receiver: Receiver<InternalSignal>, on_progress: OnProgressType) {
        let last_trigger = Mutex::new(0);
        let content_length = self.content_length().await.ok();
        match on_progress {
            OnProgressType::None => {}
            OnProgressType::Closure(closure) => {
                while let Some(data) = receiver.recv().await {
                    match data {
                        InternalSignal::Value(data) => {
                            let arguments = CallbackArguments {
                                current_chunk: data,
                                content_length,
                            };
                            closure(arguments);
                        }
                        InternalSignal::Finished => break,
                    }
                }
            }
            OnProgressType::AsyncClosure(closure) => {
                while let Some(data) = receiver.recv().await {
                    match data {
                        InternalSignal::Value(data) => {
                            let arguments = CallbackArguments {
                                current_chunk: data,
                                content_length,
                            };
                            closure(arguments).await;
                        }
                        InternalSignal::Finished => break,
                    }
                }
            }
            OnProgressType::Channel(sender, cancel_on_close) => {
                while let Some(data) = receiver.recv().await {
                    match data {
                        InternalSignal::Value(data) => {
                            let arguments = CallbackArguments {
                                current_chunk: data,
                                content_length,
                            };
                            // await if channel is full
                            if sender.send(arguments).await.is_err() && cancel_on_close {
                                receiver.close()
                            }
                        }
                        InternalSignal::Finished => break,
                    }
                }
            }
            OnProgressType::SlowClosure(closure) => {
                while let Some(data) = receiver.recv().await {
                    match data {
                        InternalSignal::Value(data) => {
                            if let Ok(mut trigger) = last_trigger.try_lock() {
                                // discard any digits beyond the million digit
                                let current_million = data / 1_000_000;
                                if *trigger < current_million {
                                    *trigger = current_million;
                                    let arguments = CallbackArguments {
                                        current_chunk: data,
                                        content_length,
                                    };
                                    closure(arguments)
                                }
                            }
                        }
                        InternalSignal::Finished => break,
                    }
                }
            }
            OnProgressType::SlowAsyncClosure(closure) => {
                while let Some(data) = receiver.recv().await {
                    match data {
                        InternalSignal::Value(data) => {
                            if let Ok(mut trigger) = last_trigger.try_lock() {
                                // discard any digits beyond the million digit
                                let current_million = data / 1_000_000;
                                if *trigger < current_million {
                                    *trigger = current_million;
                                    let arguments = CallbackArguments {
                                        current_chunk: data,
                                        content_length,
                                    };
                                    closure(arguments).await
                                }
                            }
                        }
                        InternalSignal::Finished => break,
                    }
                }
            }
            OnProgressType::SlowChannel(sender, cancel_on_close) => {
                while let Some(data) = receiver.recv().await {
                    match data {
                        InternalSignal::Value(data) => {
                            if let Ok(mut trigger) = last_trigger.try_lock() {
                                // discard any digits beyond the million digit
                                let current_million = data / 1_000_000;
                                if *trigger < current_million {
                                    *trigger = current_million;
                                    let arguments = CallbackArguments {
                                        current_chunk: data,
                                        content_length,
                                    };
                                    if sender.send(arguments).await.is_err() && cancel_on_close {
                                        receiver.close()
                                    }
                                }
                            }
                        }
                        InternalSignal::Finished => break,
                    }
                }
            }
        }
    }

    #[inline]
    async fn on_complete(on_complete: OnCompleteType, path: Option<PathBuf>) {
        match on_complete {
            OnCompleteType::None => {}
            OnCompleteType::Closure(closure) => {
                closure(path)
            }
            OnCompleteType::AsyncClosure(closure) => {
                closure(path).await
            }
        }
    }
}
