use std::fs::{create_dir_all, read, remove_file, write};
use std::io::Read;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};

use super::Result;

/// Provides common functionality for data formats
pub trait Format<T>
where
    Self: Clone + Default,
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// The default file extension for this format
    const EXT: &'static str;

    /// Attempts to encode the given value into a byte array
    fn try_into_bytes(&self, value: &T) -> Result<Vec<u8>>;
    /// Attempts to decode the given value from a byte array
    fn try_from_bytes(&self, bytes: &[u8]) -> Result<T>;

    /// Returns the format's compression level
    #[must_use]
    fn get_compression(&self) -> Compression {
        Compression::none()
    }
    /// Returns `true` if the format is compressed
    #[must_use]
    fn is_compressed(&self) -> bool {
        self.get_compression() == Compression::none()
    }
    /// Attempts to compress the given byte slice using the format's compression
    /// value
    fn try_gz_encode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(bytes, self.get_compression());
        // I don't know for sure how much the compression will help so it's better to
        // just over-allocate imo.
        let mut output = Vec::with_capacity(bytes.len());

        encoder.read_to_end(&mut output)?;

        Ok(output)
    }
    /// Attempts to decompress the given byte slice
    fn try_gz_decode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzDecoder::new(bytes);
        // I don't know for sure how much the decompression will expand the data, so the
        // best I can do without overshooting is just pass the original size and cope.
        let mut output = Vec::with_capacity(bytes.len());

        encoder.read_to_end(&mut output)?;

        Ok(output)
    }

    /// Attempts to encode the given value
    fn encode(&self, value: &T) -> Result<Vec<u8>> {
        // No point in passing it through gz if there's no compression
        if self.is_compressed() {
            self.try_gz_encode(&self.try_into_bytes(value)?)
        } else {
            self.try_into_bytes(value)
        }
    }
    /// Attempts to decode the given byte slice
    fn decode(&self, bytes: &[u8]) -> Result<T> {
        // No point in passing it through gz if there's no compression
        if self.is_compressed() {
            self.try_from_bytes(&self.try_gz_decode(bytes)?)
        } else {
            self.try_from_bytes(bytes)
        }
    }
}

/// The Message Pack format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessagePack {
    /// A plain, uncompressed messagepack format
    Plain,
    /// A standard, somewhat compressed messagepack format
    #[default]
    Standard,
    /// A dense, heavily compressed messagepack format
    Dense,
}

impl<T> Format<T> for MessagePack
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    const EXT: &'static str = "dat";

    fn try_into_bytes(&self, value: &T) -> Result<Vec<u8>> {
        Ok(rmp_serde::to_vec(value)?)
    }

    fn try_from_bytes(&self, bytes: &[u8]) -> Result<T> {
        Ok(rmp_serde::from_slice(bytes)?)
    }

    fn get_compression(&self) -> Compression {
        match self {
            Self::Plain => Compression::none(),
            Self::Standard => Compression::default(),
            Self::Dense => Compression::best(),
        }
    }
}

/// The TOML format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Toml;

impl<T> Format<T> for Toml
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    const EXT: &'static str = "toml";

    fn try_into_bytes(&self, value: &T) -> Result<Vec<u8>> {
        Ok(toml::to_string_pretty(value).map(String::into_bytes)?)
    }

    fn try_from_bytes(&self, bytes: &[u8]) -> Result<T> {
        // Any data lost within the string almost certainly means there was corruption,
        // or I'm reading the wrong format. No harm in just erroring from any
        // lost bytes, but if it works regardless that's (probably) great.
        Ok(toml::from_str(&String::from_utf8_lossy(bytes))?)
    }
}

/// Represents a reference to data within the bot's storage
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataId<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format<T>,
{
    path: PathBuf,
    format: F,
    _marker: PhantomData<T>,
}

impl<T, F> DataId<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format<T>,
{
    /// The base directory of all stored data
    pub const DIR: &str = "data";

    /// Creates a new data identifier with the provided format
    #[must_use]
    pub fn new_formatted(path: impl AsRef<Path>, format: F) -> Self {
        Self {
            path: PathBuf::from(Self::DIR).join(path).with_extension(F::EXT),
            format,
            _marker: PhantomData,
        }
    }

    /// Creates a new data identifier with a default format
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self::new_formatted(path, F::default())
    }

    /// Returns a reference to the data's path
    pub const fn get_path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns a reference to the data's format
    pub const fn get_format(&self) -> &F {
        &self.format
    }

    /// Creates a new [`Data`] wrapper
    pub const fn create(self, value: T) -> Data<T, F> {
        Data::new(self, value)
    }

    /// Reads and decodes the value referenced by the [`DataId`]
    pub fn read(self) -> Result<Data<T, F>> {
        Data::read(self)
    }
}

/// Represents data stored within the file system
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Data<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format<T>,
{
    id: DataId<T, F>,
    value: T,
}

impl<T, F> Data<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format<T>,
{
    /// Creates a new [`Data`] wrapper
    #[must_use]
    pub const fn new(id: DataId<T, F>, value: T) -> Self {
        Self { id, value }
    }

    /// Reads and decodes the value referenced by the provided [`DataId`]
    pub fn read(id: DataId<T, F>) -> Result<Self> {
        let bytes = read(id.get_path())?;
        let value = id.get_format().decode(&bytes)?;

        Ok(Self::new(id, value))
    }

    /// Returns a reference to the stored value
    pub const fn get(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the stored value
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Writes the stored value to the bot's storage, dropping the wrapper
    pub fn write(self) -> Result<DataId<T, F>> {
        let bytes = self.id.get_format().encode(self.get())?;

        // Gotta make sure the parent directories exist. Forgotten that too many times
        // at this point to accidentally leave it out and get annoyed about my data not
        // being saved.
        self.id.get_path().parent().map_or(Ok(()), create_dir_all)?;
        write(self.id.get_path(), bytes)?;

        Ok(self.id)
    }

    /// Removes the data from the bot's storage, dropping the wrapper and
    /// returning its inner values
    pub fn remove(self) -> Result<(DataId<T, F>, T)> {
        remove_file(self.id.get_path())?;

        Ok((self.id, self.value))
    }
}

/// Creates a new data identifier
#[macro_export]
macro_rules! data {
    (<$t: ty, $f: ty> $( $arg: tt )+) => {
        $crate::util::data::DataId::<$t, $f>::new(format!($( $arg )+))
    };
    (<$t: ty> $f: expr, $( $arg: tt )+) => {
        $crate::util::data::DataId::<$t, _>::new_formatted(format!($( $arg )+), $f)
    };
    (<$f: ty> $( $arg: tt )+) => {
        $crate::data!(<_, $f> $( $arg )+)
    };
    ($f: expr, $( $arg: tt )+) => {
        $crate::data!(<_> $f, $( $arg )+)
    };
}

/// Shared functionality for values stored within the file system
pub trait StoredData
where
    Self: Serialize + for<'de> Deserialize<'de>,
{
    /// The values passed to create the value's data identifier
    type Args: Serialize + for<'de> Deserialize<'de>;
    /// The data [`Format`] of the implementing value
    type Format: Format<Self>;

    /// Returns the value's data identifier
    fn data_id(args: Self::Args) -> DataId<Self, Self::Format>;

    /// Creates a new [`Data`] wrapper with the given `value`
    ///
    /// Convenience method for `StoredData::data_id(args).create(value)`
    fn data_create(args: Self::Args, value: Self) -> Data<Self, Self::Format> {
        Self::data_id(args).create(value)
    }
    /// Reads and decodes the value referenced by the [`DataId`]
    ///
    /// Convenience method for `StoredData::data_id(args).read()`
    fn data_read(args: Self::Args) -> Result<Data<Self, Self::Format>> {
        Self::data_id(args).read()
    }
    /// Reads and decodes the value referenced by the [`DataId`], automatically
    /// creating the value's default when an error occurs
    fn data_default(args: Self::Args) -> Data<Self, Self::Format>
    where
        Self: Clone + Default,
    {
        let id = Self::data_id(args);

        id.clone()
            .read()
            .unwrap_or_else(|_| id.create(Self::default()))
    }
}
