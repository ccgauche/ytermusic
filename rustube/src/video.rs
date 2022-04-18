use std::sync::Arc;

use derive_more::Display;

use crate::video_info::player_response::video_details::VideoDetails;
use crate::{Id, Stream, VideoInfo};

/// A YouTube downloader, which allows you to download all available formats and qualities of a
/// YouTube video.
///
/// Each instance of [`Video`] represents an existing, available, and downloadable
/// video.
///
/// There are two ways of constructing an instance of [`Video`]:
/// 1. By using the asynchronous `Video::from_*` methods. These methods will take some kind of
/// video-identifier, like an [`Url`] or an [`Id`], will then internally download the necessary video
/// information and finally descramble it.
/// 2. By calling [`VideoDescrambler::descramble`]. Since a [`VideoDescrambler`] already
/// contains the necessary video information, and just need to descramble it, no requests are
/// performed. (This gives you more control over the process).
///
/// # Examples
/// - Constructing using [`Video::from_url`] (or [`Video::from_id`]) (easiest way)
/// ```no_run
///# use rustube::Video;
///# use url::Url;
///# #[tokio::main]
///# async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let url = Url::parse("https://youtube.com/watch?iv=5jlI4uzZGjU")?;
/// let video: Video = Video::from_url(&url).await?;
///# Ok(())
///# }
/// ```
/// - Constructing using [`VideoDescrambler::descramble`]
/// ```no_run
///# use rustube::{Video, VideoFetcher, VideoDescrambler};
///# use url::Url;
///# #[tokio::main]
///# async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let url = Url::parse("https://youtube.com/watch?iv=5jlI4uzZGjU")?;
/// let fetcher: VideoFetcher = VideoFetcher::from_url(&url)?;
/// let descrambler: VideoDescrambler = fetcher.fetch().await?;  
/// let video: Video = descrambler.descramble()?;
///# Ok(())
///# }
/// ```
/// - Construction using chained calls
/// ```no_run
///# use rustube::{Video, VideoFetcher, VideoDescrambler};
///# use url::Url;
///# #[tokio::main]
///# async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let url = Url::parse("https://youtube.com/watch?iv=5jlI4uzZGjU")?;
/// let video: Video = VideoFetcher::from_url(&url)?
///    .fetch()
///    .await?
///    .descramble()?;
///# Ok(())
///# }
/// ```
/// - Downloading a video using an existing [`Video`] instance
/// ```no_run
///# use rustube::{Video, VideoFetcher, VideoDescrambler};
///# use url::Url;
///# #[tokio::main]
///# async fn main() -> Result<(), Box<dyn std::error::Error>> {
///# let url = Url::parse("https://youtube.com/watch?iv=5jlI4uzZGjU")?;
///# let video: Video = Video::from_url(&url).await?;
/// let video_path = video
///    .streams()
///    .iter()
///    .filter(|stream| stream.includes_video_track && stream.includes_audio_track)
///    .max_by_key(|stream| stream.quality_label)
///    .unwrap()
///    .download()
///    .await?;
///# Ok(())
///# }
/// ```
/// [`Url`]: url::Url
/// [`VideoDescrambler`]: crate::descrambler::VideoDescrambler
/// [`VideoDescrambler::descramble`]: crate::descrambler::VideoDescrambler::descramble
#[derive(Clone, Debug, Display, PartialEq)]
#[display(
    fmt = "Video({}, streams: {})",
    "video_info.player_response.video_details.video_id",
    "streams.len()"
)]
pub struct Video {
    pub(crate) video_info: VideoInfo,
    pub(crate) streams: Vec<Stream>,
}

impl Video {
    /// Creates a [`Video`] from an [`Url`](url::Url).
    /// ### Errors
    /// - When [`VideoFetcher::from_url`](crate::VideoFetcher::from_url) fails.
    /// - When [`VideoFetcher::fetch`](crate::VideoFetcher::fetch) fails.
    /// - When [`VideoDescrambler::descramble`](crate::VideoDescrambler::descramble) fails.
    #[inline]
    #[cfg(all(feature = "download", feature = "regex"))]
    #[doc(cfg(all(feature = "download", feature = "regex")))]
    pub async fn from_url(url: &url::Url) -> crate::Result<Self> {
        crate::VideoFetcher::from_url(url)?
            .fetch()
            .await?
            .descramble()
    }

    /// Creates a [`Video`] from an [`Id`].
    /// ### Errors
    /// - When [`VideoFetcher::fetch`](crate::VideoFetcher::fetch) fails.
    /// - When [`VideoDescrambler::descramble`](crate::VideoDescrambler::descramble) fails.
    #[inline]
    #[cfg(feature = "download")]
    #[doc(cfg(feature = "download"))]
    pub async fn from_id(id: crate::IdBuf) -> crate::Result<Self> {
        crate::VideoFetcher::from_id(id)?
            .fetch()
            .await?
            .descramble()
    }

    /// The [`VideoInfo`] of the video.
    #[inline]
    pub fn video_info(&self) -> &VideoInfo {
        &self.video_info
    }

    /// All [`Stream`]s of the video.
    #[inline]
    pub fn streams(&self) -> &Vec<Stream> {
        &self.streams
    }

    /// Takes all [`Stream`]s of the video.
    #[inline]
    pub fn into_streams(self) -> Vec<Stream> {
        self.streams
    }

    /// Decomposes a `Video` into it's raw parts.
    #[inline]
    pub fn into_parts(self) -> (VideoInfo, Vec<Stream>) {
        (self.video_info, self.streams)
    }

    /// The [`VideoDetails`]s of the video.
    #[inline]
    pub fn video_details(&self) -> Arc<VideoDetails> {
        Arc::clone(&self.video_info.player_response.video_details)
    }

    /// The [`Id`] of the video.
    #[inline]
    pub fn id(&self) -> Id<'_> {
        self.video_info
            .player_response
            .video_details
            .video_id
            .as_borrowed()
    }

    /// The title of the video.
    #[inline]
    pub fn title(&self) -> &str {
        self.video_info.player_response.video_details.title.as_str()
    }

    /// Whether or not the video is age restricted.
    #[inline]
    pub fn is_age_restricted(&self) -> bool {
        self.video_info.is_age_restricted
    }

    /// The [`Stream`] with the best quality.
    /// This stream is guaranteed to contain both a video as well as an audio track.
    #[inline]
    pub fn best_quality(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .filter(|stream| stream.includes_video_track && stream.includes_audio_track)
            .max_by_key(|stream| stream.quality_label)
    }

    /// The [`Stream`] with the worst quality.
    /// This stream is guaranteed to contain both a video as well as an audio track.
    #[inline]
    pub fn worst_quality(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .filter(|stream| stream.includes_video_track && stream.includes_audio_track)
            .min_by_key(|stream| stream.quality_label)
    }

    /// The [`Stream`] with the best video quality.
    /// This stream is guaranteed to contain only a video but no audio track.
    #[inline]
    pub fn best_video(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .filter(|stream| stream.includes_video_track && !stream.includes_audio_track)
            .max_by_key(|stream| stream.width)
    }

    /// The [`Stream`] with the worst video quality.
    /// This stream is guaranteed to contain only a video but no audio track.
    #[inline]
    pub fn worst_video(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .filter(|stream| stream.includes_video_track && !stream.includes_audio_track)
            .min_by_key(|stream| stream.width)
    }

    /// The [`Stream`] with the best audio quality.
    /// This stream is guaranteed to contain only a audio but no video track.    
    #[inline]
    pub fn best_audio(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .filter(|stream| {
                stream.mime.to_string() == "audio/mp4"
                    && stream.includes_audio_track
                    && !stream.includes_video_track
            })
            .map(|x| {
                x
            })
            .max_by_key(|stream| stream.bitrate)
    }

    /// The [`Stream`] with the worst audio quality.
    /// This stream is guaranteed to contain only a audio but no video track.
    #[inline]
    pub fn worst_audio(&self) -> Option<&Stream> {
        self.streams
            .iter()
            .filter(|stream| stream.includes_audio_track && !stream.includes_video_track)
            .min_by_key(|stream| stream.bitrate)
    }
}
