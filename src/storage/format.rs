use std::io::Write;

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};

use crate::utility::Result;

/// Provides common functionality for data formats.
pub trait Format: Copy + Default + Send + Sync {
    /// The format's expected file extension.
    ///
    /// This will be automatically appended to file paths when resolving data
    /// keys.
    const EXT: &'static str;

    /// Encodes the provided value into a heap-allocated byte array.
    fn encode<T>(&self, value: &T) -> Result<Box<[u8]>>
    where
        T: Serialize;

    /// Decodes the referenced byte slice into a value.
    fn decode<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;
}

/// A format that automatically handles data compression.
///
/// Using this format wrapper provides two guarantees, provided the generic `C`
/// constant is non-zero:
/// - All encoded bytes will be compressed after serialization.
/// - All decoded bytes will be decompressed before deserialization.
///
/// The generic `C` constant should be within 0-9, inclusively, with `0`
/// representing no compression and `9` representing the slowest (and best)
/// compression level.
///
/// The default `C` compression level is set to `6`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Zip<F: Format, const C: u8 = 6>(F);

impl<F: Format, const C: u8> Zip<F, C> {
    /// Creates a new [`Zip`] format containing the provided inner format value.
    #[inline]
    pub const fn new(format: F) -> Self {
        Self(format)
    }

    /// Returns a reference to the [`Zip`]'s inner format value.
    #[inline]
    pub const fn wrapped(&self) -> &F {
        &self.0
    }

    /// Returns the [`Zip`]'s compression level.
    #[inline]
    pub const fn compression(&self) -> Compression {
        Compression::new(C as u32)
    }
}

impl<F: Format, const C: u8> Format for Zip<F, C> {
    const EXT: &'static str = "dat";

    fn encode<T>(&self, value: &T) -> Result<Box<[u8]>>
    where
        T: Serialize,
    {
        let bytes = self.wrapped().encode(value)?;
        let buffer = Vec::with_capacity(bytes.len());
        let mut encoder = GzEncoder::new(buffer, self.compression());

        encoder.write_all(&bytes)?;

        Ok(encoder.finish()?.into_boxed_slice())
    }

    fn decode<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let buffer = Vec::with_capacity(bytes.len());
        let mut decoder = GzDecoder::new(buffer);

        decoder.write_all(bytes)?;

        self.wrapped().decode(&decoder.finish()?)
    }
}

/// The [MessagePack](https://msgpack.org/index.html) data format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MessagePack;

impl Format for MessagePack {
    const EXT: &'static str = "pack";

    fn encode<T>(&self, value: &T) -> Result<Box<[u8]>>
    where
        T: Serialize,
    {
        Ok(rmp_serde::to_vec(value)?.into_boxed_slice())
    }

    fn decode<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}

/// The [JSON](https://www.json.org/json-en.html) data format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Json;

impl Format for Json {
    const EXT: &'static str = "json";

    fn encode<T>(&self, value: &T) -> Result<Box<[u8]>>
    where
        T: Serialize,
    {
        Ok(serde_json::to_vec_pretty(value)?.into_boxed_slice())
    }

    fn decode<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        Ok(serde_json::from_slice(bytes)?)
    }
}

/// The [TOML](https://toml.io/en/) data format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Toml;

impl Format for Toml {
    const EXT: &'static str = "toml";

    fn encode<T>(&self, value: &T) -> Result<Box<[u8]>>
    where
        T: Serialize,
    {
        Ok(toml::to_string_pretty(value)?
            .into_boxed_str()
            .into_boxed_bytes())
    }

    fn decode<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let string = String::from_utf8(bytes.to_vec())?;

        Ok(toml::from_str(string.as_str())?)
    }
}
