use std::io::Write;

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};

use crate::{Error, Format};

/// A [`Format`] that automatically compresses and decompresses data.
///
/// Using this wrapper ensures that all values will, assuming that `C` is non-zero:
/// - Be compressed *after* encoding.
/// - Be decompressed *before* decoding.
///
/// The generic constant `C` should be assigned a value between `0`-`9`, with `0` representing no
/// compression and `9` representing the slowest (but best) compression level. By default, `C` is
/// assigned to `5` for a middle-of-the-road, average compression level.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Compress<F: Format, const C: u8 = 5>(F);

impl<F: Format, const C: u8> Compress<F, C> {
    /// Creates a new [`Compress<F, C>`].
    pub const fn new(format: F) -> Self {
        Self(format)
    }

    /// Returns a reference to the inner [`Format`] of this [`Compress<F, C>`].
    pub const fn inner(&self) -> &F {
        &self.0
    }

    /// Returns the [`Compression`] level of this [`Compress<F, C>`].
    pub const fn compression(&self) -> Compression {
        debug_assert!(C <= 9);

        Compression::new(C as u32)
    }
}

impl<F: Format, const C: u8> Format for Compress<F, C> {
    type EncodingError = Error<F>;
    type DecodingError = Error<F>;

    fn extension(&self) -> String {
        self.0.extension() + "x"
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::EncodingError> {
        let bytes = self.inner().encode(value).map_err(Error::Encoding)?;
        let buffer = Vec::with_capacity(bytes.len());
        let mut encoder = GzEncoder::new(buffer, self.compression());

        encoder.write_all(&bytes)?;
        encoder.finish().map_err(Into::into)
    }

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::DecodingError> {
        let buffer = Vec::with_capacity(bytes.len());
        let mut decoder = GzDecoder::new(buffer);

        decoder.write_all(bytes)?;
        self.inner().decode(&decoder.finish()?).map_err(Error::Decoding)
    }
}
