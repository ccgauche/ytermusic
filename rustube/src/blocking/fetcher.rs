use std::ops::{Deref, DerefMut};

use reqwest::Client;
use url::Url;

use crate::{IdBuf, Result};
use crate::blocking::descrambler::VideoDescrambler;
use crate::fetcher::VideoFetcher as AsyncVideoFetcher;

/// A synchronous wrapper around [`VideoFetcher`](crate::VideoFetcher).
#[derive(Clone, Debug, derive_more::Display, PartialEq, Eq)]
pub struct VideoFetcher(AsyncVideoFetcher);

impl VideoFetcher {
    /// Constructs a [`VideoFetcher`] from an `Url`.
    /// ### Errors
    /// - When [`Id::from_raw`](crate::Id) fails to extracted the videos id from the url.
    /// - When [`reqwest`] fails to initialize an new [`Client`].
    #[inline]
    pub fn from_url(url: &Url) -> Result<Self> {
        Ok(Self(AsyncVideoFetcher::from_url(url)?))
    }

    /// Constructs a [`VideoFetcher`] from an `Id`.
    /// ### Errors
    /// When [`reqwest`] fails to initialize an new [`Client`].
    #[inline]
    pub fn from_id(video_id: IdBuf) -> Result<Self> {
        Ok(Self(AsyncVideoFetcher::from_id(video_id)?))
    }

    /// Constructs a [`VideoFetcher`] from an [`Id`](crate::Id) and an existing [`Client`].
    /// There are no special constrains, what the [`Client`] has to look like.
    #[inline]
    pub fn from_id_with_client(video_id: IdBuf, client: Client) -> Self {
        Self(AsyncVideoFetcher::from_id_with_client(video_id, client))
    }

    /// A synchronous wrapper around [`VideoFetcher::fetch`](crate::VideoFetcher::fetch).
    #[inline]
    pub fn fetch(self) -> Result<VideoDescrambler> {
        Ok(VideoDescrambler(block!(self.0.fetch())?))
    }
}

impl Deref for VideoFetcher {
    type Target = AsyncVideoFetcher;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VideoFetcher {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
