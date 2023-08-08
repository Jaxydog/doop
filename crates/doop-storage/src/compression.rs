use std::fmt::Display;
use std::io::Write;

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};

use crate::Format;

/// An error that may occur during compression or decompression.
#[derive(Debug, thiserror::Error)]
pub enum CompressError<F>
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// An error during IO reading / writing.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error during compression.
    #[error(transparent)]
    Compress(#[from] flate2::CompressError),
    /// An error during decompression.
    #[error(transparent)]
    Decompress(#[from] flate2::DecompressError),
    /// An error during encoding.
    #[error("{0}")]
    Encode(F::EncodeError),
    /// An error during decoding.
    #[error("{0}")]
    Decode(F::DecodeError),
}

/// A file format that automatically compresses and decompresses data.
///
/// Using this wrapper ensures that all values will, assuming that the generic constant `C` is
/// non-zero:
/// - Be compressed *after* encoding.
/// - Be de-compressed *before* decoding.
///
/// The generic constant `C` should be assigned a value between 0-9, with 0 representing no
/// compression and 9 representing the slowest (and best) compression level. By default, `C` is
/// assigned to 5 for a middle-of-the-road, average compression level.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Compress<F, const C: u8 = 5>(F)
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display;

impl<F, const C: u8> Compress<F, C>
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// Creates a new [`Compress<T, F, C>`].
    #[inline]
    pub const fn new(format: F) -> Self { Self(format) }

    /// Returns a reference to the inner format of this [`Compress<T, F, C>`].
    #[inline]
    pub const fn inner(&self) -> &F { &self.0 }

    /// Returns the compression level of this [`Compress<T, F, C>`].
    #[allow(clippy::unused_self)]
    #[inline]
    pub const fn compression(&self) -> Compression { Compression::new(C as u32) }
}

#[cfg(feature = "default-systems")]
impl<F, const C: u8> crate::systems::FileFormat for Compress<F, C>
where
    F: crate::systems::FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    fn extension(&self) -> String { self.inner().extension() + ".z" }
}

impl<F, const C: u8> Format for Compress<F, C>
where
    F: Format,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    type EncodeError = CompressError<F>;
    type DecodeError = CompressError<F>;

    fn encode<T>(&self, value: &T) -> Result<Vec<u8>, Self::EncodeError>
    where
        T: Serialize,
    {
        let bytes = self.inner().encode(value).map_err(CompressError::Encode)?;
        let buffer = Vec::with_capacity(bytes.len());
        let mut encoder = GzEncoder::new(buffer, self.compression());

        encoder.write_all(&bytes)?;

        encoder.finish().map_err(Into::into)
    }

    fn decode<T>(&self, bytes: &[u8]) -> Result<T, Self::DecodeError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let buffer = Vec::with_capacity(bytes.len());
        let mut decoder = GzDecoder::new(buffer);

        decoder.write_all(bytes)?;

        self.inner().decode(&decoder.finish()?).map_err(CompressError::Decode)
    }
}
