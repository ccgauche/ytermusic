use std::ops::{Deref, DerefMut};

use crate::blocking::video::Video;
use crate::descrambler::VideoDescrambler as AsyncVideoDescrambler;
use crate::Result;

/// A synchronous wrapper around [`VideoDescrambler`](crate::VideoDescrambler).
#[derive(Clone, Debug, derive_more::Display, PartialEq, Eq)]
pub struct VideoDescrambler(pub(super) AsyncVideoDescrambler);

impl VideoDescrambler {
    /// A synchronous wrapper around [`VideoDescrambler::descramble`](crate::VideoDescrambler::descramble).
    #[inline]
    pub fn descramble(self) -> Result<Video> {
        Ok(Video(self.0.descramble()?))
    }
}

impl Deref for VideoDescrambler {
    type Target = AsyncVideoDescrambler;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VideoDescrambler {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
