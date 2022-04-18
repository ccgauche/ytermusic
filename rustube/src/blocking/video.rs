use std::ops::{Deref, DerefMut};

use crate::{Stream, Video as AsyncVideo};

/// A synchronous wrapper around [`Video`](crate::Video).
#[derive(Clone, Debug, derive_more::Display, PartialEq)]
pub struct Video(pub(super) AsyncVideo);

impl Video {
    /// A synchronous wrapper around [`Video::form_url`](crate::Video::from_url).
    /// 
    /// Creates a [`Video`] from an [`Url`](url::Url).
    /// ### Errors
    /// - When [`VideoFetcher::from_url`](crate::VideoFetcher::from_url) fails.
    /// - When [`VideoFetcher::fetch`](crate::VideoFetcher::fetch) fails.
    /// - When [`VideoDescrambler::descramble`](crate::VideoDescrambler::descramble) fails.
    #[inline]
    #[cfg(all(feature = "download", feature = "regex"))]
    pub fn from_url(url: &url::Url) -> crate::Result<Self> {
        Ok(Self(block!(AsyncVideo::from_url(url))?))
    }


    /// A synchronous wrapper around [`Video::form_id`](crate::Video::from_id).
    ///
    /// Creates a [`Video`] from an [`Id`](crate::Id).
    /// ### Errors
    /// - When [`VideoFetcher::fetch`](crate::VideoFetcher::fetch) fails.
    /// - When [`VideoDescrambler::descramble`](crate::VideoDescrambler::descramble) fails.
    #[inline]
    #[cfg(feature = "download")]
    pub fn from_id(id: crate::IdBuf) -> crate::Result<Self> {
        Ok(Self(block!(AsyncVideo::from_id(id))?))
    }

    /// Takes all [`Stream`]s of the video.
    #[inline]
    pub fn into_streams(self) -> Vec<Stream> {
        self.0.streams
    }
}

impl Deref for Video {
    type Target = AsyncVideo;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Video {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
