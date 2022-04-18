use alloc::borrow::Cow;

/// Errors that can occur during the id extraction or the video download process.   
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("the provided raw Id does not match any known Id-pattern")]
    BadIdFormat,
    #[cfg(feature = "fetch")]
    #[doc(cfg(feature = "fetch"))]
    #[error("the video you requested is unavailable:\n{0:#?}")]
    VideoUnavailable(Box<crate::video_info::player_response::playability_status::PlayabilityStatus>),
    #[cfg(feature = "download")]
    #[doc(cfg(feature = "download"))]
    #[error("the video contains no streams")]
    NoStreams,

    #[error(transparent)]
    #[cfg(feature = "fetch")]
    #[doc(cfg(feature = "fetch"))]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    #[cfg(feature = "fetch")]
    #[doc(cfg(feature = "fetch"))]
    Request(#[from] reqwest::Error),
    #[error("YouTube returned an unexpected response: `{0}`")]
    UnexpectedResponse(Cow<'static, str>),
    #[error(transparent)]
    #[cfg(feature = "fetch")]
    #[doc(cfg(feature = "fetch"))]
    QueryDeserialization(#[from] serde_qs::Error),
    #[error(transparent)]
    #[cfg(feature = "fetch")]
    #[doc(cfg(feature = "fetch"))]
    JsonDeserialization(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("{0}")]
    Custom(Cow<'static, str>),
    #[error("a potentially dangerous error occurred: {0}")]
    Fatal(String),
    #[error(
    "the error, which occurred is not meant an error, but is used for internal comunication.\
            This error should never be propagated to the public API."
    )]
    Internal(&'static str),
    #[error("The internal channel has been closed")]
    #[cfg(feature = "callback")]
    #[doc(cfg(feature = "callback"))]
    ChannelClosed,
}
