#![feature(
    doc_cfg,
    async_closure,
    bool_to_option,
    cow_is_borrowed,
    once_cell,
    box_syntax,
    str_split_as_str,
    option_result_contains
)]
#![allow(clippy::nonstandard_macro_braces)]
#![warn(
// missing_docs,
rust_2018_idioms,
unreachable_pub
)]
#![deny(broken_intra_doc_links)]
#![doc(test(no_crate_inject))]
#![cfg_attr(not(any(feature = "std", feature = "regex")), no_std)]

//! A complete (WIP), and easy to use YouTube downloader.
//!
//! ## Just show me the code!
//! You just want to download a video, and don't care about any intermediate steps and any video
//! information?
//!
//! That's it:
//! ```no_run
//!# #[tokio::main]
//!# async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let url = "https://www.youtube.com/watch?v=Edx9D2yaOGs&ab_channel=CollegeHumor";
//! let path_to_video = rustube::download_best_quality(url).await?;
//!# Ok(())
//!# }
//! ```
//! And with the `blocking` feature enabled, you don't even have to bring your own runtime:
//! ```no_run
//!# fn main() -> Result<(), Box<dyn std::error::Error>> {
//!# #[cfg(feature = "blocking")]
//!# {
//! let url = "https://youtu.be/nv2wQvn6Wxc";
//! let path_to_video = rustube::blocking::download_best_quality(url)?;
//!# }
//!# Ok(())
//!# }
//! ```
//!
//! ## Getting video information
//! Of course, there's also the use case, where you want to find out information about a video,
//! like it's [view count], it's [title], or if it [is_unplugged_corpus] (I mean who of us doesn't
//! have the desire to find that out).
//!
//! In these cases, straigt out using [`download_best_quality`] won't serve you well.
//! The [`VideoDescrambler`] returned by [`VideoFetcher::fetch`] will probaply fit your usecase a
//! lot better:
//! ```no_run
//!# use rustube::{Id, VideoFetcher};
//!# #[tokio::main]
//!# async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let id = Id::from_raw("https://www.youtube.com/watch?v=bKldI-XGHIw")?;
//! let descrambler = VideoFetcher::from_id(id.into_owned())?
//!    .fetch()
//!    .await?;
//!
//! let video_info = descrambler.video_info();
//! let the_only_truth = &video_info.player_response.tracking_params;
//!# Ok(())
//!# }
//! ```
//! If, after finding out everything about a video, you suddenly decide downloading it is worth it,
//! you, of curse, can keep using the [`VideoDescrambler`] for that:
//! ```no_run
//!# use rustube::{Id, VideoFetcher};
//!# #[tokio::main]
//!# async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!# let id = Id::from_raw("https://www.youtube.com/watch?v=bKldI-XGHIw")?;
//!# let descrambler = VideoFetcher::from_id(id.into_owned())?
//!#    .fetch()
//!#    .await?;
//! let video = descrambler.descramble()?;
//! let path_to_video = video.best_quality().unwrap().download().await?;
//!# Ok(())
//!# }
//! ```  
//!
//! ## Maybe something in between?
//! So then, what does `rustube` offer, if I already know, that I want information as well as
//! downloading the video? That's exactly, what the handy `from_*` methods on [`Video`] are for.
//! Those methods provide easy to use shortcuts with no need for first fetching and
//! then descrambeling the video seperatly:
//!```no_run
//!# use rustube::{Video, Id};
//!# #[tokio::main]
//!# async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let id = Id::from_str("hFZFjoX2cGg")?;
//! let video = Video::from_id(id.into_owned()).await?;
//!
//! let the_truth_the_whole_truth_and_nothing_but_the_truth = video.video_info();
//! let path_to_video = video
//!    .worst_audio()
//!    .unwrap()
//!    .download()
//!    .await?;
//!# Ok(())
//!# }
//!```
//!
//! ## Choosing something exotic
//! Till now, you only saw the methods [`Video::best_quality`] and [`Video::worst_audio`] that
//! magically tell you which video stream you truly desire. But wait, what's a [`Stream`]? If you
//! ever watched a video on YouTube, you probably know that most videos come in different
//! resolutions. So when your internet connection sucks, you may watch the 240p version, instead of
//! the full fleged 4k variant. Each of those resolutions is a [`Stream`]. Besides those video
//! [`Stream`]s, there are often also video-only or audio-only [`Stream`]s. The methods we used so
//! far are actually just a nice shortcut for making your life easier. But since all these success
//! gurus tell us, we should take the hard road, we will!
//!
//! For doing so, and to get a little more control over which [`Stream`] of a [`Video`] to download,
//! we can use [`Video::streams`], the [`Stream`] attributes, and Rusts amazing [`Iterator`] methods:
//! ```no_run
//!# #[tokio::main]
//!# async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!# use rustube::{Video, Id};
//!# let id = Id::from_str("hFZFjoX2cGg")?;
//!# let video = Video::from_id(id.into_owned()).await?;
//! let best_quality = video
//!    .streams()
//!    .iter()
//!    .filter(|stream| stream.includes_video_track && stream.includes_audio_track)
//!    .max_by_key(|stream| stream.quality_label);
//!# Ok(())
//!# }
//!```
//!
//! Note, that often the video-audio streams have slightly worse quality than the video-only or
//! audio-only streams. This is not a limitation of rustube, but rather a weird design choice of
//! YouTube. So if you want the absolute best quality possible, you will sometimes have to download
//! the best video-only and the best audio-only stream separate. This, however, doesn't affect all
//! videos.
//!
//! ## Different ways of downloading
//! As you may already have noticed, all the above examples just call [`Stream::download`], and then
//! get back a path to a video. This path will always point to `<VIDEO_ID>.mp4` in the current
//! working directory. But what if you want to have a little more control over where
//! to download the video to?
//!
//! [`Stream::download_to_dir`] and [`Stream::download_to`] have your back! Those methods allow you
//! to specify exactly, where the video should be downloaded too.
//!
//! If you want to do something, while the download is progressing, enable the `callback`
//! feature and use the then availabe `*_with_callback` methods, like
//! [`Stream::download_with_callback`].
//!
//! The [`Callback`] struct can take up to one `on_progress` method and one `on_complete` method.
//!
//! For even more control, or when you just want to access the videos URL, have a look at the
//! [`url`](crate::video_info::player_response::streaming_data::SignatureCipher::url) field inside
//! of [`Stream::signature_cipher`]. This field contains the video URL of that particular Stream.
//! You can, i.e., use this URL to watch the video directly in your browser.  
//!
//! ## Feature flags
//! One of the goals of `rustube` is to eventually deserialize the complete video information, so
//! even the weirdest niche cases get all the information they need. Another goal is to become the
//! fastest and best performing YouTube downloader out there, while also using little resources.
//! These two goals don't go hand in hand and require `rustube` to offer some kind of feature
//! system, which allows users to specify precisely, what they need, so they don't have to pay the
//! price for stuff they don't use.
//!
//! When compiling with no features at all, you will be left with only [`Id`]. This is a `no_std`
//! build. Still, it's highly recommended to at least enable the `regex` feature, which will
//! currently break `no_std` ([#476](https://github.com/rust-lang/regex/issues/476)), as well as the
//! `std` feature. This combination enables [`Id::from_raw`], which is the only way of extracting
//! ids from arbitrary video identifiers, like URLs.
//!
//! The feature system is still WIP, and currently, you can just opt-in or opt-out of quite huge
//! bundles of functionality.
//!
//! - `download`: \[default\] Enables all utilities required for downloading videos.
//! - `regex`: \[default\] Enables [`Id::from_raw`], which extracts valid `Id`s from arbitrary video
//!   identifiers like URLs.
//! - `serde`: \[default\] Enables [`serde`] support for [`Id`] (Keep in mind, that this feature
//!   does not enable the `regex` automatically).
//! - `std`: \[default\] Enables `std` usage, which a lot of things depend on.
//! - `fetch`: \[default\] Enables [`VideoFetcher`], which can be used to fetch video information.
//! - `descramble`: \[default\] Enables [`VideoDescrambler`], which can decrypt video signatures and is
//!   necessary to extract the individual streams.
//! - `stream`: \[default\] Enables [`Stream`], a representation of a video stream that can be used to download this particular stream.
//! - `blocking`: Enables the [`blocking`] API, which internally creates a [`tokio`] runtime for you
//!   , so you don't have to care about it yourself. (Keep in mind, that this feature does not enable
//!   any of the other features above automatically)
//! - `callback`: Enables to add callbacks to downlaods and the [`Callback`] struct itself
//!
//!
//! [view count]: crate::video_info::player_response::video_details::VideoDetails::view_count
//! [title]: crate::video_info::player_response::video_details::VideoDetails::title
//! [is_unplugged_corpus]: crate::video_info::player_response::video_details::VideoDetails::is_unplugged_corpus
//! [Iterator]: std::iter::Iterator

extern crate alloc;

#[cfg(feature = "tokio")]
#[doc(cfg(feature = "tokio"))]
pub use tokio;
pub use url;

#[cfg(feature = "std")]
#[doc(cfg(feature = "std"))]
pub use crate::error::Error;
pub use crate::id::Id;
#[cfg(feature = "descramble")]
#[doc(cfg(feature = "descramble"))]
pub use crate::video::Video;
#[doc(inline)]
#[cfg(feature = "fetch")]
#[doc(cfg(feature = "fetch"))]
pub use crate::video_info::{
    player_response::{video_details::VideoDetails, PlayerResponse},
    VideoInfo,
};

/// Alias for `Result`, with the default error type [`Error`].
#[cfg(feature = "std")]
#[doc(cfg(feature = "std"))]
type Result<T, E = Error> = core::result::Result<T, E>;

#[doc(hidden)]
#[cfg(feature = "descramble")]
#[doc(cfg(feature = "descramble"))]
mod descrambler;
#[doc(hidden)]
#[cfg(feature = "std")]
#[doc(cfg(feature = "std"))]
pub mod error;
#[doc(hidden)]
#[cfg(feature = "fetch")]
#[doc(cfg(feature = "fetch"))]
mod fetcher;
#[doc(hidden)]
mod id;
#[doc(hidden)]
#[cfg(feature = "stream")]
#[doc(cfg(feature = "stream"))]
mod stream;
#[doc(hidden)]
#[cfg(feature = "descramble")]
#[doc(cfg(feature = "descramble"))]
mod video;
#[cfg(feature = "fetch")]
#[doc(cfg(feature = "fetch"))]
mod video_info;

#[cfg(feature = "fetch")]
mod serde_impl;

/// A trait for collecting iterators into arbitrary, in particular fixed-sized, types.
trait TryCollect<T>: Iterator {
    fn try_collect(self) -> Option<T>;
    fn try_collect_lossy(self) -> Option<T>
    where
        Self: Sized,
    {
        None
    }
}

impl<T> TryCollect<(T::Item,)> for T
where
    T: Iterator,
{
    #[inline]
    fn try_collect(mut self) -> Option<(T::Item,)> {
        match (self.next(), self.next()) {
            (Some(item), None) => Some((item,)),
            _ => None,
        }
    }

    #[inline]
    fn try_collect_lossy(mut self) -> Option<(T::Item,)> {
        self.next().map(|v| (v,))
    }
}

impl<T> TryCollect<(T::Item, T::Item)> for T
where
    T: Iterator,
{
    #[inline]
    fn try_collect(mut self) -> Option<(T::Item, T::Item)> {
        match (self.next(), self.next(), self.next()) {
            (Some(item1), Some(item2), None) => Some((item1, item2)),
            _ => None,
        }
    }

    #[inline]
    fn try_collect_lossy(mut self) -> Option<(T::Item, T::Item)> {
        match (self.next(), self.next()) {
            (Some(item1), Some(item2)) => Some((item1, item2)),
            _ => None,
        }
    }
}
