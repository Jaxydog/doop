use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::format::Format;
use crate::utility::Result;

/// Provides definitions for distinguishing between various storage formats.
pub mod format;

/// Represents an identifier for a value that is saved between processes.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Info<T, F>
where
    T: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// The inner file location.
    path: Box<Path>,
    /// The inner storage format.
    format: F,
    /// The inner value type marker.
    _marker: PhantomData<T>,
}

impl<T, F> Info<T, F>
where
    T: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// The directory that stores all saved data.
    const DIR: &str = "data";

    /// Creates a new [`Info`] identifier with the given format.
    pub fn new_in(path: impl AsRef<Path>, format: F) -> Self {
        let path = PathBuf::from(Self::DIR).join(path).with_extension(F::EXT);

        Self { path: path.into_boxed_path(), format, _marker: PhantomData }
    }

    /// Creates a new [`Info`] identifier with a default format.
    #[inline]
    pub fn new(path: impl AsRef<Path>) -> Self { Self::new_in(path, F::default()) }

    /// Returns a reference to the inner file path.
    #[inline]
    pub const fn path(&self) -> &Path { &self.path }

    /// Returns a reference to the inner format.
    #[inline]
    pub const fn format(&self) -> &F { &self.format }

    /// Reads the saved value from the file system.
    #[inline]
    pub async fn read(self) -> Result<Stored<T, F>> { Stored::read(self).await }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    pub async fn read_or(self, value: T) -> Stored<T, F>
    where
        T: Clone,
    {
        Stored::read_or(self, value).await
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    pub async fn read_or_else(self, f: impl Send + FnOnce() -> T) -> Stored<T, F>
    where
        T: Clone,
    {
        Stored::read_or_else(self, f).await
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    pub async fn read_or_default(self) -> Stored<T, F>
    where
        T: Clone + Default,
    {
        Stored::read_or_default(self).await
    }

    /// Writes the inner value into the file system, unwrapping and returning
    /// the contained [`Info`] information.
    #[inline]
    pub async fn write(self, value: T) -> Result<Self> { Stored::new(self, value).write().await }

    /// Removes the inner value from the file system, unwrapping and returning
    /// the contained [`Info`] information and the associated value.
    #[inline]
    pub async fn remove(self) -> Result<(Self, T)> { self.read().await?.remove().await }
}

/// Represents a value that persists between processes and its associated
/// [`Info`] identifier.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stored<T, F>
where
    T: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// The inner [`Info`] identifier.
    info: Info<T, F>,
    /// The inner value.
    value: T,
}

impl<T, F> Stored<T, F>
where
    T: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    F: Format,
{
    /// Creates a new [`Stored`] value associated to the given [`Info`].
    #[inline]
    pub const fn new(info: Info<T, F>, value: T) -> Self { Self { info, value } }

    /// Creates a new [`Stored`] value assigned to the [`Info`] identifier
    /// constructed from the provided arguments.
    #[inline]
    pub fn new_from(path: impl AsRef<Path>, value: T) -> Self { Self::new(Info::new(path), value) }

    /// Creates a new [`Stored`] value assigned to the [`Info`] identifier
    /// constructed from the provided arguments.
    #[inline]
    pub fn new_from_in(path: impl AsRef<Path>, format: F, value: T) -> Self {
        Self::new(Info::new_in(path, format), value)
    }

    /// Returns a reference to the associated [`Info`] information.
    #[inline]
    pub const fn info(&self) -> &Info<T, F> { &self.info }

    /// Returns a reference to the inner value.
    #[inline]
    pub const fn get(&self) -> &T { &self.value }

    /// Returns a mutable reference to the inner value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T { &mut self.value }

    /// Returns a the inner value, dropping the container.
    #[inline]
    pub fn get_owned(self) -> T { self.value }

    /// Reads the saved value from the file system.
    pub async fn read(info: Info<T, F>) -> Result<Self> {
        let bytes = tokio::fs::read(info.path()).await?;
        let value = info.format().decode(&bytes)?;

        Ok(Self::new(info, value))
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    pub async fn read_or(info: Info<T, F>, value: T) -> Self
    where
        T: Clone,
    {
        Self::read(info.clone())
            .await
            .unwrap_or_else(|_| Self::new(info, value))
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    pub async fn read_or_else(info: Info<T, F>, f: impl Send + FnOnce() -> T) -> Self
    where
        T: Clone,
    {
        Self::read(info.clone())
            .await
            .unwrap_or_else(|_| Self::new(info, f()))
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    pub async fn read_or_default(info: Info<T, F>) -> Self
    where
        T: Clone + Default,
    {
        Self::read_or_else(info, T::default).await
    }

    /// Writes the inner value into the file system, unwrapping and returning
    /// the contained [`Info`] information.
    pub async fn write(self) -> Result<Info<T, F>> {
        let bytes = self.info().format().encode(self.get())?;

        if let Some(path) = self.info().path().parent() {
            tokio::fs::create_dir_all(path).await?;
        }

        tokio::fs::write(self.info().path(), bytes).await?;

        Ok(self.info)
    }

    /// Removes the inner value from the file system, unwrapping and returning
    /// the contained [`Info`] information and the associated value.
    pub async fn remove(self) -> Result<(Info<T, F>, T)> {
        let Self { info, value } = self;

        tokio::fs::remove_file(info.path()).await?;

        Ok((info, value))
    }
}

/// Provides an interface for reading and writing persisted data to and from the
/// file system using [`Info`] information and [`Stored`] values.
#[async_trait::async_trait]
pub trait Storable: Sized + Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// The arguments provided when creating [`Info`] information.
    type Arguments: Send + Sync;
    /// The value's storage format.
    type Format: Format;

    /// Returns the implementing type's associated [`Info`] information for the
    /// provided arguments.
    fn saved(arguments: Self::Arguments) -> Info<Self, Self::Format>;

    /// Reads the saved value from the file system.
    #[inline]
    async fn read(arguments: Self::Arguments) -> Result<Stored<Self, Self::Format>> {
        Self::saved(arguments).read().await
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    async fn read_or(arguments: Self::Arguments, value: Self) -> Stored<Self, Self::Format>
    where
        Self: Clone,
    {
        Self::saved(arguments).read_or(value).await
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    async fn read_or_else(
        arguments: Self::Arguments,
        f: impl Send + FnOnce() -> Self,
    ) -> Stored<Self, Self::Format>
    where
        Self: Clone,
    {
        Self::saved(arguments).read_or_else(f).await
    }

    /// Reads the saved value from the file system, providing a default value
    /// when an error occurs.
    #[inline]
    async fn read_or_default(arguments: Self::Arguments) -> Stored<Self, Self::Format>
    where
        Self: Clone + Default,
    {
        Self::saved(arguments).read_or_default().await
    }

    /// Writes the value into the file system, returning the value's [`Info`]
    /// information.
    #[inline]
    async fn write(self, arguments: Self::Arguments) -> Result<Info<Self, Self::Format>> {
        Self::saved(arguments).write(self).await
    }

    /// Removes the value from the file system, returning the value's [`Info`]
    /// information and the stored value.
    #[inline]
    async fn remove(arguments: Self::Arguments) -> Result<(Info<Self, Self::Format>, Self)> {
        Self::saved(arguments).remove().await
    }
}
