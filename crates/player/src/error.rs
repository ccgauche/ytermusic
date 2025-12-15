// Custom Error Enum to handle different failures
#[derive(Debug)]
pub enum PlayError {
    Io(std::io::Error),
    DecoderError(rodio::decoder::DecoderError),
    StreamError(rodio::StreamError),
    PlayError(rodio::PlayError),
    SeekError(rodio::source::SeekError),
}

impl From<rodio::PlayError> for PlayError {
    fn from(err: rodio::PlayError) -> Self {
        PlayError::PlayError(err)
    }
}
