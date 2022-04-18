use std::sync::Arc;

use reqwest::Client;
use url::Url;

use cipher::Cipher;

use crate::{IdBuf, Stream, Video, VideoDetails, VideoInfo};
use crate::error::Error;
use crate::video_info::player_response::streaming_data::RawFormat;
use crate::video_info::player_response::streaming_data::StreamingData;

mod cipher;

/// A descrambler used to decrypt the data fetched by [`VideoFetcher`].
///
/// You will probably rarely use this type directly, and use [`Video`] instead. 
/// There's no public way of directly constructing a [`VideoDescrambler`]. The only way of getting
/// one is by calling [`VideoFetcher::fetch`].
///
/// # Example
/// ```no_run
///# use rustube::{VideoFetcher, Id, VideoDescrambler};
///# use url::Url;
/// let url = Url::parse("https://youtube.com/watch?iv=5jlI4uzZGjU").unwrap();
/// 
///# tokio_test::block_on(async {
/// let fetcher: VideoFetcher =  VideoFetcher::from_url(&url).unwrap();
/// let descrambler: VideoDescrambler = fetcher.fetch().await.unwrap();
///# }); 
/// ``` 
/// 
/// # How it works
/// (To fully understand `descramble`, you should first read how [`VideoFetcher`] works).
/// 
/// Descrambling, in this case, mainly refers to descrambling the [`SignatureCipher`]. After we 
/// requested the [`VideoInfo`] in `fetch`, we are left with many [`RawFormat`]s. A [`RawFormat`] is 
/// just a bucket full of information about a video. Those formats come in two flavours: pre-signed 
/// and encrypted formats. Pre-signed formats are actually a free lunch. Such formats already 
/// contain a valid video URL, which can be used to download the video. The encrypted once are a 
/// little bit more complicated.
///
/// These encrypted [`RawFormat`]s contain a so called [`SignatureCipher`] with a the signature 
/// field [`s`] in it. This signature is a long string and the YouTube server requires us to 
/// include in the URL query or we get a `403` back. Unfortunalty this signature isn't correct yet!
/// We first need to decrypt it. And that's where the `transform_plan` and the `transform_map` come
/// into play.   
/// 
/// The `transform_plan` is just a list of JavaScript function calls, which take a string (or an 
/// array) plus sometimes an integer as input. The called JavaScript functions then transforms the 
/// string in a certain way and returns a new string. This new string then represents the new 
/// signature. To decrypt the signature we just need to pass it through all of these functions in a
/// row.
/// 
/// But wait! How can we run JavaScript in Rust? And doesn't that come with a considerable overhead?
/// It actually would come with a vast overhead! That's why we need the `transform_map`. The 
/// `transform_map` is a `HashMap<String, TransformFn>`, which maps JavaScript function names to
/// Rust functions.
///
/// To finally decrypt the signature, we just iterate over each function call in the the
/// `transform_plan`, extract both the function name and the optinal integer argument, and call the 
/// corresponding Rust function in `transform_map`.
/// 
/// The last step `descramble` performs, is to take all [`RawFormat`]s, which now contain the 
/// correct signature, and convert them to [`Stream`]s. At the end of the day, `Stream`s are just
/// `RawFormat`s with some extra information.
/// 
/// And that's it! We can now download a YouTube video like we would download any other
/// video from the internet. The only difference is that the [`Stream`]s [`url`]
/// will eventually expire.
/// 
/// [`SignatureCipher`]: crate::video_info::player_response::streaming_data::SignatureCipher
/// [`s`]: crate::video_info::player_response::streaming_data::SignatureCipher::s
/// [`url`]: crate::video_info::player_response::streaming_data::SignatureCipher::url
/// [`VideoFetcher::fetch`]: crate::fetcher::VideoFetcher::fetch
/// [`VideoFetcher`]: crate::fetcher::VideoFetcher
/// [`VideoFetcher::fetch`]: crate::fetcher::VideoFetcher::fetch
#[derive(Clone, derive_more::Display, derivative::Derivative)]
#[display(fmt = "VideoDescrambler({})", "video_info.player_response.video_details.video_id")]
#[derivative(Debug, PartialEq, Eq)]
pub struct VideoDescrambler {
    pub(crate) video_info: VideoInfo,
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub(crate) client: Client,
    pub(crate) js: String,
}

impl VideoDescrambler {
    /// Descrambles the data fetched by YouTubeFetcher.
    /// For more information have a look at the [`Video`] documentation.
    ///
    /// ### Errors
    /// - When the streaming data of the video is incomplete.
    /// - When descrambling the videos signatures fails.
    #[log_derive::logfn(ok = "Trace", err = "Error")]
    #[log_derive::logfn_inputs(Trace)]
    pub fn descramble(mut self) -> crate::Result<Video> {
        let streaming_data = self.video_info.player_response.streaming_data
            .as_mut()
            .ok_or_else(|| Error::Custom(
                "VideoInfo contained no StreamingData, which is essential for downloading.".into()
            ))?;

        if let Some(ref adaptive_fmts_raw) = self.video_info.adaptive_fmts_raw {
            // fixme: this should probably be part of fetch.
            apply_descrambler_adaptive_fmts(streaming_data, adaptive_fmts_raw)?;
        }

        apply_signature(streaming_data, &self.js)?;
        let mut streams = Vec::new();
        Self::initialize_streams(
            streaming_data,
            &mut streams,
            &self.client,
            &self.video_info.player_response.video_details,
        );

        Ok(Video {
            video_info: self.video_info,
            streams,
        })
    }

    /// The [`VideoInfo`] of the video.
    #[inline]
    pub fn video_info(&self) -> &VideoInfo {
        &self.video_info
    }

    /// The [`VideoDetails`] of the video.
    #[inline]
    pub fn video_details(&self) -> &VideoDetails {
        &self.video_info.player_response.video_details
    }

    /// The [`Id`](crate::Id) of the video.
    #[inline]
    pub fn video_id(&self) -> &IdBuf {
        &self.video_details().video_id
    }

    /// The title of the video.
    #[inline]
    pub fn video_title(&self) -> &String {
        &self.video_details().title
    }

    /// Consumes all [`RawFormat`]s and constructs [`Stream`]s from them. 
    #[inline]
    fn initialize_streams(
        streaming_data: &mut StreamingData,
        streams: &mut Vec<Stream>,
        client: &Client,
        video_details: &Arc<VideoDetails>,
    ) {
        for raw_format in streaming_data.formats.drain(..).chain(streaming_data.adaptive_formats.drain(..)) {
            let stream = Stream::from_raw_format(
                raw_format,
                client.clone(),
                Arc::clone(video_details),
            );
            streams.push(stream);
        }
    }
}

/// Extracts the [`RawFormat`]s from `adaptive_fmts_raw`. (This may be a legacy thing) 
#[inline]
fn apply_descrambler_adaptive_fmts(streaming_data: &mut StreamingData, adaptive_fmts_raw: &str) -> crate::Result<()> {
    for raw_fmt in adaptive_fmts_raw.split(',') {
        // fixme: this implementation is likely wrong. 
        // main question: is adaptive_fmts_raw a list of normal RawFormats?
        // To make is correct, I would need sample data for adaptive_fmts_raw
        log::warn!(
            "`apply_descrambler_adaptive_fmts` is probaply broken!\
             Please open an issue on GitHub and paste in the whole warning message (it may be quite long).\
             adaptive_fmts_raw: `{}`", raw_fmt
        );
        let raw_format = serde_qs::from_str::<RawFormat>(raw_fmt)?;
        streaming_data.formats.push(raw_format);
    }

    Ok(())
}

/// Descrambles the signature of a video.
#[inline]
fn apply_signature(streaming_data: &mut StreamingData, js: &str) -> crate::Result<()> {
    let cipher = Cipher::from_js(js)?;

    for raw_format in streaming_data.formats.iter_mut().chain(streaming_data.adaptive_formats.iter_mut()) {
        let url = &mut raw_format.signature_cipher.url;
        let s = match raw_format.signature_cipher.s {
            Some(ref mut s) => s,
            None if url_already_contains_signature(url) => continue,
            None => return Err(Error::UnexpectedResponse(
                "RawFormat did not contain a signature (s), nor did the url".into()
            ))
        };

        cipher.decrypt_signature(s)?;
        url
            .query_pairs_mut()
            .append_pair("sig", s);
    }

    Ok(())
}

/// Checks whether or not the video url is already signed.
#[inline]
fn url_already_contains_signature(url: &Url) -> bool {
    let url = url.as_str();
    url.contains("signature") || (url.contains("&sig=") || url.contains("&lsig="))
}
