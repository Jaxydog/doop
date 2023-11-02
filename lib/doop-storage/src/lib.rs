//! Provides a data storage interface for the Doop Discord bot.
#![deny(clippy::expect_used, unsafe_code, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::todo, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(feature = "compress")] pub use crate::compress::*;
#[cfg(feature = "compress")] mod compress;

#[cfg(feature = "encrypt")] pub use crate::encrypt::*;
#[cfg(feature = "encrypt")] mod encrypt;

#[cfg(feature = "formats")] pub use crate::formats::*;
#[cfg(feature = "formats")] mod formats;

/// A possible error.
#[derive(Debug, thiserror::Error)]
pub enum Error<F: Format> {
    /// An IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A [`Format`] encoding error.
    #[error("{0}")]
    Encoding(F::EncodingError),
    /// A [`Format`] decoding error.
    #[error("{0}")]
    Decoding(F::DecodingError),
}

/// A data storage [`Format`] used within the storage system.
///
/// The purpose of this trait is to provide dynamic (and chainable) [`encode`](<Format::encode>) and
/// [`decode`](<Format::decode>) method implementations. While there are no restrictions for
/// implementing [`Format`], it's recommended to avoid large and / or exceedingly complex types.
///
/// Most formats *should* generally:
/// - Be zero-sized or `#[repr(transparent)]` newtype structs or enums.
/// - Have "simple" [`encode`](<Format::encode>) and [`decode`](<Format::decode>) implementations.
/// - Provide a `new` method or implement [`Default`].
pub trait Format: Debug {
    /// The type returned in the event of an error during encoding.
    type EncodingError: Debug + Display;
    /// The type returned in the event of an error during decoding.
    type DecodingError: Debug + Display;

    /// Returns the extension for this [`Format`].
    fn extension(&self) -> String;

    /// Encodes a given value of type `T` into a byte array.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be encoded.
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::EncodingError>;

    /// Decodes a given byte slice into a value of type `T`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the bytes could not be decoded.
    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::DecodingError>;
}

/// Describes and represents a resource entry within the filesystem.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Key<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// The value's filepath.
    path: Box<Path>,
    /// The value's format.
    format: F,
    /// Type marker.
    _marker: PhantomData<fn() -> T>,
}

impl<T, F> Key<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// Creates a new [`Key<T, F>`].
    pub fn new(path: impl AsRef<Path>, format: F) -> Self {
        Self { path: Box::from(path.as_ref()), format, _marker: PhantomData }
    }

    /// Creates a new [`Key<T, F>`] with a defaulted format.
    pub fn new_default(path: impl AsRef<Path>) -> Self
    where
        F: Default,
    {
        Self::new(path, F::default())
    }

    /// Returns whether this [`Key<T, F>`] exists within the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path could not be verified.
    pub fn exists(&self) -> Result<bool, Error<F>> {
        self.path.try_exists().map_err(Into::into)
    }

    /// Reads this [`Key<T, F>`]'s associated resource.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be read or decoded.
    pub fn read(&self) -> Result<Value<T, F>, Error<F>> {
        let bytes = std::fs::read(&(*self.path))?;
        let value = self.format.decode(&bytes).map_err(Error::Decoding)?;

        Ok(Value { key: self, value })
    }

    /// Reads this [`Key<T, F>`]'s associated resource, providing a default value if it fails.
    pub fn read_or(&self, value: T) -> Value<T, F> {
        self.read().unwrap_or(Value { key: self, value })
    }

    /// Reads this [`Key<T, F>`]'s associated resource, providing a default value from the given
    /// closure if it fails.
    pub fn read_or_else(&self, f: impl FnOnce() -> T) -> Value<T, F> {
        self.read().unwrap_or_else(|_| Value { key: self, value: f() })
    }

    /// Reads this [`Key<T, F>`]'s associated resource, providing the default value if it fails.
    pub fn read_or_default(&self) -> Value<T, F>
    where
        T: Default,
    {
        self.read_or_else(T::default)
    }

    /// Writes the given value into this [`Key<T, F>`]'s associated resource.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be encoded or written.
    pub fn write(&self, value: &T) -> Result<(), Error<F>> {
        let bytes = self.format.encode(value).map_err(Error::Encoding)?;

        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        std::fs::write(&(*self.path), bytes)?;

        Ok(())
    }

    /// Removes the resource associated with this [`Key<T, F>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the resource could not be removed.
    pub fn remove(&self) -> Result<(), Error<F>> {
        std::fs::remove_file(&(*self.path)).map_err(Into::into)
    }
}

impl<T, F, S> From<S> for Key<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format + Default,
    S: AsRef<Path>,
{
    fn from(value: S) -> Self {
        Self::new_default(value.as_ref())
    }
}

/// Describes a resource that exists within the file system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value<'key, T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// The value's associated key.
    key: &'key Key<T, F>,
    /// The stored value.
    value: T,
}

impl<'key, T, F> Value<'key, T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// Returns a reference to the associated key of this [`Value<T, F>`].
    pub const fn key(&self) -> &Key<T, F> {
        self.key
    }

    /// Returns a reference to the inner value of this [`Value<T, F>`].
    pub const fn get(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the inner value of this [`Value<T, F>`].
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Returns the inner value of this [`Value<T, F>`].
    pub fn get_owned(self) -> T {
        self.value
    }

    /// Writes the given value into this associated [`Value<T, F>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be encoded or written.
    pub fn write(&self) -> Result<(), Error<F>> {
        self.key.write(&self.value)
    }
}

/// Provides a data storage key builder for the implementing type.
pub trait Stored: Serialize + for<'de> Deserialize<'de> {
    /// The arguments provided when creating a new [`Key<T, F>`].
    type Arguments;
    /// The expected [`Format`] of this type.
    type Format: Format;

    /// Creates a new [`Key<T, F>`] with the provided arguments.
    fn stored(arguments: Self::Arguments) -> Key<Self, Self::Format>;
}
