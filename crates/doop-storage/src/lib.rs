#![doc = env!("CARGO_PKG_DESCRIPTION")]
#![forbid(clippy::panic, clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};

#[cfg(feature = "compression")] pub use crate::compression::*;
#[cfg(feature = "encryption")] pub use crate::encryption::*;
#[cfg(feature = "default-formats")] pub use crate::formats::*;
#[cfg(feature = "default-systems")] pub use crate::systems::*;

#[cfg(feature = "compression")] mod compression;
#[cfg(feature = "encryption")] mod encryption;
#[cfg(feature = "default-formats")] mod formats;
#[cfg(feature = "default-systems")] mod systems;

/// An interface and implementation of a storage system.
///
/// The [`System<T, F>`] trait represents an interface into a database or storage medium. It
/// provides abstractions over external methods to allow data to be easily read from and written to
/// various types of storage.
pub trait System<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format + ?Sized,
{
    /// The type returned in the event of an error.
    type Error;
    /// The [`System<T, F>`]'s [`Key<T, F>`] implementation.
    type Key: Key<T, F, Val = Self::Val>;
    /// The [`System<T, F>`]'s [`Val<T, F>`] implementation.
    type Val: Val<T, F, Key = Self::Key>;

    /// Returns a reference to the instance of this [`System<T, F>`].
    fn instance<'s>() -> &'s Self;

    /// Returns whether the stored data at the given [`Key<T, F>`] exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the system was unable to access the data.
    fn exists(&self, key: Self::Key) -> Result<(Self::Key, bool), Self::Error>;

    /// Reads the stored data at the given [`Key<T, F>`] from the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be read or decoded.
    fn read(&self, key: Self::Key) -> Result<Self::Val, Self::Error>;

    /// Writes the given [`Val<T, F>`] at the associated [`Key<T, F>`] into the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be written or encoded.
    fn write(&self, val: Self::Val) -> Result<Self::Key, Self::Error>;

    /// Removes the stored data at the given [`Key<T, F>`] from the storage system, returning the
    /// previously stored [`Val<T, F>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the data does not exist or could not be removed.
    fn remove(&self, key: Self::Key) -> Result<Self::Val, Self::Error>;
}

/// Represents a key into a storage [`System<T, F>`].
///
/// Each implementation of a [`Key<T, F>`] must have an associated [`Val<T, F>`] implementation.
pub trait Key<T, F>: Sized
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format + ?Sized,
{
    /// This [`Key<T, F>`]'s [`System<T, F>`] implementation.
    type System: System<T, F, Key = Self, Val = Self::Val>;
    /// This [`Key<T, F>`]'s [`Val<T, F>`] implementation.
    type Val: Val<T, F, System = Self::System, Key = Self>;

    /// Assigns the given data to this [`Key<T, F>`], returning a new [`Val<T, F>`].
    fn assign(self, data: T) -> Self::Val;

    /// Returns a reference to the [`Format`] of this [`Key<T, F>`].
    fn format(&self) -> &F;

    /// Returns whether the stored data at this [`Key<T, F>`] exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the system was unable to access the data.
    #[inline]
    fn exists(self) -> Result<(Self, bool), <Self::System as System<T, F>>::Error> {
        Self::System::instance().exists(self)
    }

    /// Reads the stored data at this [`Key<T, F>`] from the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be read or decoded.
    #[inline]
    fn read(self) -> Result<Self::Val, <Self::System as System<T, F>>::Error> {
        Self::System::instance().read(self)
    }

    /// Reads the stored data at this [`Key<T, F>`] from the storage system, returning the given
    /// default data if an error is encountered.
    #[inline]
    fn read_or(self, data: T) -> Self::Val
    where
        Self: Clone,
    {
        self.read_or_else(|| data)
    }

    /// Reads the stored data at this [`Key<T, F>`] from the storage system, returning the given
    /// closure's returned data if an error is encountered.
    #[inline]
    fn read_or_else(self, f: impl FnOnce() -> T) -> Self::Val
    where
        Self: Clone,
    {
        let key = self;

        Self::System::instance().read(key.clone()).unwrap_or_else(|_| key.assign(f()))
    }

    /// Reads the stored data at this [`Key<T, F>`] from the storage system, returning the default
    /// data if an error is encountered.
    #[inline]
    fn read_or_default(self) -> Self::Val
    where
        T: Default,
        Self: Clone,
    {
        self.read_or_else(T::default)
    }

    /// Removes the stored data at the [`Key<T, F>`] from the storage system, returning the
    /// previously stored [`Val<T, F>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the data does not exist or could not be removed.
    #[inline]
    fn remove(self) -> Result<Self::Val, <Self::System as System<T, F>>::Error> {
        Self::System::instance().remove(self)
    }
}

/// Represents a value within a storage [`System<T, F>`].
///
/// Each implementation of a [`Val<T, F>`] must have an associated [`Key<T, F>`] implementation.
pub trait Val<T, F>: Sized
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Format + ?Sized,
{
    /// This [`Key<T, F>`]'s [`System<T, F>`] implementation.
    type System: System<T, F, Key = Self::Key, Val = Self>;
    /// This [`Key<T, F>`]'s [`Key<T, F>`] implementation.
    type Key: Key<T, F, System = Self::System, Val = Self>;

    /// Returns a reference to the [`Key<T, F>`] of this [`Val<T, F>`].
    fn key(&self) -> &Self::Key;

    /// Returns a reference to the [`Key<T, F>`] of this [`Val<T, F>`].
    fn key_owned(self) -> Self::Key;

    /// Returns a reference to the data of this [`Val<T, F>`].
    fn data(&self) -> &T;

    /// Returns a mutable reference to the data of this [`Val<T, F>`].
    fn data_mut(&mut self) -> &mut T;

    /// Returns the data of this [`Val<T, F>`].
    fn data_owned(self) -> T;

    /// Writes the [`Val<T, F>`] at the associated [`Key<T, F>`] into the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be written or encoded.
    #[inline]
    fn write(self) -> Result<Self::Key, <Self::System as System<T, F>>::Error> {
        Self::System::instance().write(self)
    }
}

/// A data storage [`Format`] used within a storage [`System<T, F>`].
///
/// The purpose of this trait is to provide dynamic (and chainable) [`encode`](<Format::encode>) and
/// [`decode`](<Format::decode>) method implementations through the type system.
///
/// While there are no restrictions on implementing [`Format`], it's recommended to avoid large or
/// exceeding complex types. Most formats *should* generally:
/// 1. Be transparent or newtype structs or enums.
/// 2. Have "simple" [`encode`](<Format::encode>) and [`decode`](<Format::decode>) methods.
/// 3. Provide a `new`-like method or implement [`Default`].
pub trait Format {
    /// The type returned in the event of an error during encoding.
    type EncodeError;
    /// The type returned in the event of an error during decoding.
    type DecodeError;

    /// Encodes a given value of type `T` into a byte array.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be encoded.
    fn encode<T>(&self, value: &T) -> Result<Vec<u8>, Self::EncodeError>
    where
        T: Serialize;

    /// Decodes a given byte array into a value of type `T`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the bytes could not be decoded.
    fn decode<T>(&self, bytes: &[u8]) -> Result<T, Self::DecodeError>
    where
        T: for<'de> Deserialize<'de>;
}

/// Data that can be stored within a storage system.
pub trait Storage<S, K, V>
where
    Self: Serialize + for<'de> Deserialize<'de>,
    S: System<Self, Self::Format, Key = K, Val = V>,
    K: Key<Self, Self::Format, System = S, Val = V>,
    V: Val<Self, Self::Format, System = S, Key = K>,
{
    /// The arguments provided when creating a new [`Key<T, F>`].
    type Arguments;
    /// The expected [`Format`] of this type.
    type Format: Format;

    /// Creates a new storage [`Key<T, F>`] using the provided arguments.
    fn stored(arguments: Self::Arguments) -> K;

    /// Returns whether the stored data at the created [`Key<T, F>`] exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the system was unable to access the data.
    #[inline]
    fn exists(arguments: Self::Arguments) -> Result<(K, bool), S::Error> {
        Self::stored(arguments).exists()
    }

    /// Reads the stored data at the created [`Key<T, F>`] from the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be read or decoded.
    #[inline]
    fn read(arguments: Self::Arguments) -> Result<V, S::Error> { Self::stored(arguments).read() }

    /// Reads the stored data at the created [`Key<T, F>`] from the storage system, returning the
    /// given default data if an error is encountered.
    #[inline]
    fn read_or(arguments: Self::Arguments, data: Self) -> V
    where
        K: Clone,
    {
        Self::stored(arguments).read_or(data)
    }

    /// Reads the stored data at the created [`Key<T, F>`] from the storage system, returning the
    /// given closure's returned data if an error is encountered.
    #[inline]
    fn read_or_else(arguments: Self::Arguments, f: impl FnOnce() -> Self) -> V
    where
        K: Clone,
    {
        Self::stored(arguments).read_or_else(f)
    }

    /// Reads the stored data at the created [`Key<T, F>`] from the storage system, returning the
    /// default data if an error is encountered.
    #[inline]
    fn read_or_default(arguments: Self::Arguments) -> V
    where
        Self: Default,
        K: Clone,
    {
        Self::stored(arguments).read_or_default()
    }

    /// Writes the created [`Val<T, F>`] at the associated [`Key<T, F>`] into the storage system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be written or encoded.
    #[inline]
    fn write(arguments: Self::Arguments, data: Self) -> Result<K, S::Error> {
        Self::stored(arguments).assign(data).write()
    }

    /// Removes the stored data at the created [`Key<T, F>`] from the storage system, returning the
    /// previously stored [`Val<T, F>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the data does not exist or could not be removed.
    #[inline]
    fn remove(arguments: Self::Arguments) -> Result<V, S::Error> {
        Self::stored(arguments).remove()
    }
}
