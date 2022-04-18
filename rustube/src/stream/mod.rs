use std::ops::Range;
#[cfg(feature = "download")]
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use mime::Mime;
use reqwest::Client;
use serde_with::{serde_as, DisplayFromStr};
#[cfg(feature = "callback")]
use tokio::sync::mpsc::error::TrySendError;
#[cfg(feature = "download")]
use tokio::{fs::File, io::AsyncWriteExt};
#[cfg(feature = "download")]
use tokio_stream::StreamExt;

#[cfg(all(feature = "callback", feature = "stream", feature = "blocking"))]
use callback::Callback;
#[cfg(feature = "callback")]
use callback::{InternalSender, InternalSignal};

use crate::{
    video_info::player_response::streaming_data::{
        AudioQuality, ColorInfo, FormatType, ProjectionType, Quality, QualityLabel, RawFormat,
        SignatureCipher,
    },
    VideoDetails,
};
#[cfg(feature = "download")]
use crate::{Error, Result};

#[cfg(feature = "callback")]
#[doc(cfg(feature = "callback"))]
pub mod callback;

// todo:
//  there are different types of streams: video, audio, and video + audio
//  make Stream and RawFormat an enum, so there are less options in it

#[cfg(all(not(feature = "callback"), feature = "download"))]
type InternalSender = ();

/// A downloadable video Stream, that contains all the important information.
#[serde_as]
#[derive(Clone, derivative::Derivative, serde::Deserialize, serde::Serialize)]
#[derivative(Debug, PartialEq)]
pub struct Stream {
    #[serde_as(as = "DisplayFromStr")]
    pub mime: Mime,
    pub codecs: Vec<String>,
    pub is_progressive: bool,
    pub includes_video_track: bool,
    pub includes_audio_track: bool,
    pub format_type: Option<FormatType>,
    pub approx_duration_ms: Option<u64>,
    pub audio_channels: Option<u8>,
    pub audio_quality: Option<AudioQuality>,
    pub audio_sample_rate: Option<u64>,
    pub average_bitrate: Option<u64>,
    pub bitrate: Option<u64>,
    pub color_info: Option<ColorInfo>,
    #[derivative(PartialEq(compare_with = "atomic_u64_is_eq"))]
    content_length: Arc<AtomicU64>,
    pub fps: u8,
    pub height: Option<u64>,
    pub high_replication: Option<bool>,
    pub index_range: Option<Range<u64>>,
    pub init_range: Option<Range<u64>>,
    pub is_otf: bool,
    pub itag: u64,
    pub last_modified: DateTime<Utc>,
    pub loudness_db: Option<f64>,
    pub projection_type: ProjectionType,
    pub quality: Quality,
    pub quality_label: Option<QualityLabel>,
    pub signature_cipher: SignatureCipher,
    pub width: Option<u64>,
    pub video_details: Arc<VideoDetails>,
    #[allow(dead_code)]
    #[serde(skip)]
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    client: Client,
}

impl Stream {
    // maybe deserialize RawFormat seeded with client and VideoDetails
    pub(crate) fn from_raw_format(
        raw_format: RawFormat,
        client: Client,
        video_details: Arc<VideoDetails>,
    ) -> Self {
        Self {
            is_progressive: is_progressive(&raw_format.mime_type.codecs),
            includes_video_track: includes_video_track(
                &raw_format.mime_type.codecs,
                &raw_format.mime_type.mime,
            ),
            includes_audio_track: includes_audio_track(
                &raw_format.mime_type.codecs,
                &raw_format.mime_type.mime,
            ),
            mime: raw_format.mime_type.mime,
            codecs: raw_format.mime_type.codecs,
            format_type: raw_format.format_type,
            approx_duration_ms: raw_format.approx_duration_ms,
            audio_channels: raw_format.audio_channels,
            audio_quality: raw_format.audio_quality,
            audio_sample_rate: raw_format.audio_sample_rate,
            average_bitrate: raw_format.average_bitrate,
            bitrate: raw_format.bitrate,
            color_info: raw_format.color_info,
            content_length: Arc::new(AtomicU64::new(raw_format.content_length.unwrap_or(0))),
            fps: raw_format.fps,
            height: raw_format.height,
            high_replication: raw_format.high_replication,
            index_range: raw_format.index_range,
            init_range: raw_format.init_range,
            is_otf: raw_format.format_type.contains(&FormatType::Otf),
            itag: raw_format.itag,
            last_modified: raw_format.last_modified,
            loudness_db: raw_format.loudness_db,
            projection_type: raw_format.projection_type,
            quality: raw_format.quality,
            quality_label: raw_format.quality_label,
            signature_cipher: raw_format.signature_cipher,
            width: raw_format.width,
            client,
            video_details,
        }
    }
}

// todo: download in ranges
// todo: blocking download

#[cfg(feature = "download")]
#[doc(cfg(feature = "download"))]
impl Stream {
    /// The content length of the video.
    /// If the content length was not included in the [`RawFormat`], this method will make a `HEAD`
    /// request, to try to figure it out.
    ///
    /// ### Errors:
    /// - When the content length was not included in the [`RawFormat`], and the request fails.
    #[inline]
    pub async fn content_length(&self) -> Result<u64> {
        let cl = self.content_length.load(Ordering::SeqCst);
        if cl != 0 {
            return Ok(cl);
        }

        self.client
            .head(self.signature_cipher.url.as_str())
            .send()
            .await?
            .error_for_status()?
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|cl| cl.to_str().ok())
            .and_then(|cl| cl.parse::<u64>().ok())
            .map(|cl| {
                log::trace!("content length of {:?} is {}", self, cl);
                self.content_length.store(cl, Ordering::SeqCst);
                cl
            })
            .ok_or_else(|| {
                Error::UnexpectedResponse(
                    "the response did not contain a valid content-length field".into(),
                )
            })
    }

    /// Attempts to downloads the [`Stream`]s resource.
    /// This will download the video to <video_id>.mp4 in the current working directory.
    #[inline]
    pub async fn download(&self, inpath: &Path) -> Result<PathBuf> {
        self.internal_download(inpath, None).await
    }

    #[inline]
    async fn internal_download(
        &self,
        inpath: &Path,
        channel: Option<InternalSender>,
    ) -> Result<PathBuf> {
        let path = Path::join(
            inpath,
            Path::new(self.video_details.video_id.as_str())
                .with_extension(self.mime.subtype().as_str()),
        );
        self.internal_download_to(&path, channel).await
    }

    /// Attempts to downloads the [`Stream`]s resource.
    /// This will download the video to <video_id>.mp4 in the provided directory.
    #[inline]
    pub async fn download_to_dir<P: AsRef<Path>>(&self, dir: P) -> Result<PathBuf> {
        self.internal_download_to_dir(dir, None).await
    }

    #[inline]
    async fn internal_download_to_dir<P: AsRef<Path>>(
        &self,
        dir: P,
        channel: Option<InternalSender>,
    ) -> Result<PathBuf> {
        let mut path = dir.as_ref().join(self.video_details.video_id.as_str());
        path.set_extension(self.mime.subtype().as_str());
        self.internal_download_to(&path, channel).await
    }

    /// Attempts to downloads the [`Stream`]s resource.
    /// This will download the video to the provided file path.
    #[inline]
    pub async fn download_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let _ = self.internal_download_to(path, None).await?;
        Ok(())
    }

    #[allow(unused_mut)]
    async fn internal_download_to<P: AsRef<Path>>(
        &self,
        path: P,
        channel: Option<InternalSender>,
    ) -> Result<PathBuf> {
        log::trace!("download_to: {:?}", path.as_ref());
        log::debug!("start downloading {}", self.video_details.video_id);
        let mut file = File::create(&path).await?;

        let result = match self
            .download_full(&self.signature_cipher.url, &mut file, &channel, 0)
            .await
        {
            Ok(_) => {
                log::info!(
                    "downloaded {} successfully to {:?}",
                    self.video_details.video_id,
                    path.as_ref()
                );
                log::debug!("downloaded stream {:?}", &self);
                Ok(())
            }
            Err(Error::Request(e)) if e.status().contains(&reqwest::StatusCode::NOT_FOUND) => {
                log::error!(
                    "failed to download {}: {:?}",
                    self.video_details.video_id,
                    e
                );
                log::info!(
                    "try to download {} using sequenced download",
                    self.video_details.video_id
                );
                // Some adaptive streams need to be requested with sequence numbers
                self.download_full_seq(&mut file, &channel)
                    .await
                    .map_err(|e| {
                        log::error!(
                            "failed to download {} using sequenced download: {:?}",
                            self.video_details.video_id,
                            e
                        );
                        e
                    })
            }
            Err(e) => {
                log::error!(
                    "failed to download {}: {:?}",
                    self.video_details.video_id,
                    e
                );
                drop(file);
                tokio::fs::remove_file(path.as_ref()).await?;
                Err(e)
            }
        }
        .map(|_| path.as_ref().to_path_buf());

        #[cfg(feature = "callback")]
        if let Some(channel) = channel {
            let _ = channel.send(InternalSignal::Finished).await;
        }

        result
    }

    async fn download_full_seq(
        &self,
        file: &mut File,
        channel: &Option<InternalSender>,
    ) -> Result<()> {
        // fixme: this implementation is **not** tested yet!
        // To test it, I would need an url of a video, which does require sequenced downloading.
        log::warn!(
            "`download_full_seq` is not tested yet and probably broken!\n\
            Please open a GitHub issue (https://github.com/DzenanJupic/rustube/issues) and paste \
            the whole warning message in:\n\
            id: {}\n\
            url: {}",
            self.video_details.video_id,
            self.signature_cipher.url.as_str()
        );

        let mut url = self.signature_cipher.url.clone();
        let base_query = url.query().map(str::to_owned).unwrap_or_else(String::new);

        // The 0th sequential request provides the file headers, which tell us
        // information about how the file is segmented.
        Self::set_url_seq_query(&mut url, &base_query, 0);
        let res = self.get(&url).await?;
        let segment_count = Stream::extract_segment_count(&res)?;
        // No callback action since this is not really part of the progress
        self.write_stream_to_file(res.bytes_stream(), file, &None, 0)
            .await?;
        let mut count = 0;

        for i in 1..segment_count {
            Self::set_url_seq_query(&mut url, &base_query, i);
            count = self.download_full(&url, file, channel, count).await?;
        }

        Ok(())
    }

    #[inline]
    async fn download_full(
        &self,
        url: &url::Url,
        file: &mut File,
        channel: &Option<InternalSender>,
        count: usize,
    ) -> Result<usize> {
        let res = self.get(url).await?;
        self.write_stream_to_file(res.bytes_stream(), file, channel, count)
            .await
    }

    #[inline]
    async fn get(&self, url: &url::Url) -> Result<reqwest::Response> {
        log::trace!("get: {}", url.as_str());
        Ok(self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?)
    }

    #[inline]
    #[allow(unused_variables, unused_mut)]
    async fn write_stream_to_file(
        &self,
        mut stream: impl tokio_stream::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
        file: &mut File,
        channel: &Option<InternalSender>,
        mut counter: usize,
    ) -> Result<usize> {
        // Counter will be 0 if callback is not enabled
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            log::trace!("received {} byte chunk ", chunk.len());

            file.write_all(&chunk).await?;
            #[cfg(feature = "callback")]
            if let Some(channel) = &channel {
                // network chunks of ~10kb size
                counter += chunk.len();
                // Will abort if the receiver is closed
                // Will ignore if the channel is full and thus not slow down the download
                if let Err(TrySendError::Closed(_)) =
                    channel.try_send(InternalSignal::Value(counter))
                {
                    return Err(Error::ChannelClosed);
                }
            }
        }
        Ok(counter)
    }

    #[inline]
    fn set_url_seq_query(url: &mut url::Url, base_query: &str, sq: u64) {
        url.set_query(Some(base_query));
        url.query_pairs_mut().append_pair("sq", &sq.to_string());
    }

    #[inline]
    fn extract_segment_count(res: &reqwest::Response) -> Result<u64> {
        res.headers()
            .get("Segment-Count")
            .ok_or_else(|| {
                Error::UnexpectedResponse(
                    "sequence download request did not contain a Segment-Count".into(),
                )
            })?
            .to_str()
            .map_err(|_| Error::UnexpectedResponse("Segment-Count is not valid utf-8".into()))?
            .parse::<u64>()
            .map_err(|_| {
                Error::UnexpectedResponse(
                    "Segment-Count could not be parsed into an integer".into(),
                )
            })
    }
}

#[cfg(all(feature = "download", feature = "blocking"))]
#[doc(cfg(all(feature = "download", feature = "blocking")))]
impl Stream {
    /// A synchronous wrapper around [`Stream::download`](crate::Stream::download).
    #[inline]
    pub fn blocking_download(&self) -> Result<PathBuf> {
        crate::block!(self.download())
    }

    /// A synchronous wrapper around [`Stream::download_with_callback`](crate::Stream::download_with_callback).
    #[cfg(feature = "callback")]
    #[doc(cfg(feature = "callback"))]
    #[inline]
    pub fn blocking_download_with_callback(&self, callback: Callback) -> Result<PathBuf> {
        crate::block!(self.download_with_callback(callback))
    }

    /// A synchronous wrapper around [`Stream::download_to_dir`](crate::Stream::download_to_dir).
    #[inline]
    pub fn blocking_download_to_dir<P: AsRef<Path>>(&self, dir: P) -> Result<PathBuf> {
        crate::block!(self.download_to_dir(dir))
    }

    /// A synchronous wrapper around [`Stream::download_to_dir_with_callback`](crate::Stream::download_to_dir_with_callback).
    #[cfg(feature = "callback")]
    #[doc(cfg(feature = "callback"))]
    #[inline]
    pub fn blocking_download_to_dir_with_callback<P: AsRef<Path>>(
        &self,
        dir: P,
        callback: Callback,
    ) -> Result<PathBuf> {
        crate::block!(self.download_to_dir_with_callback(dir, callback))
    }

    /// A synchronous wrapper around [`Stream::download_to`](crate::Stream::download_to).
    pub fn blocking_download_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        crate::block!(self.download_to(path))
    }

    /// A synchronous wrapper around [`Stream::download_to_with_callback`](crate::Stream::download_to_with_callback).
    #[cfg(feature = "callback")]
    #[doc(cfg(feature = "callback"))]
    pub fn blocking_download_to_with_callback<P: AsRef<Path>>(
        &self,
        path: P,
        callback: Callback,
    ) -> Result<()> {
        crate::block!(self.download_to_with_callback(path, callback))
    }

    /// A synchronous wrapper around [`Stream::content_length`](crate::Stream::content_length).
    #[inline]
    pub fn blocking_content_length(&self) -> Result<u64> {
        crate::block!(self.content_length())
    }
}

#[inline]
fn is_adaptive(codecs: &[String]) -> bool {
    codecs.len() % 2 != 0
}

#[inline]
fn includes_video_track(codecs: &[String], mime: &Mime) -> bool {
    is_progressive(codecs) || mime.type_() == "video"
}

#[inline]
fn includes_audio_track(codecs: &[String], mime: &Mime) -> bool {
    is_progressive(codecs) || mime.type_() == "audio"
}

#[inline]
fn is_progressive(codecs: &[String]) -> bool {
    !is_adaptive(codecs)
}

#[inline]
fn atomic_u64_is_eq(lhs: &Arc<AtomicU64>, rhs: &Arc<AtomicU64>) -> bool {
    lhs.load(Ordering::Acquire) == rhs.load(Ordering::Acquire)
}
