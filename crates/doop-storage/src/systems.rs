use std::fmt::Display;
use std::marker::PhantomData;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Format, Key, System, Val};

doop_macros::global! {
    /// The file-system-based [`System<T, F>`] implementation.
    static SYSTEM: FileSystem = FileSystem::default();
}

/// Initializes the file-based storage system within the given directory.
///
/// # Panics
///
/// Panics if the storage directory has already been initialized.
#[allow(clippy::expect_used)]
#[inline]
pub fn install_into(directory: impl AsRef<Path>) {
    SYSTEM
        .set(FileSystem::new(directory))
        .expect("the file-based storage system has already been initialized");
}

/// Extends a [`Format`] with file-system-specific methods.
pub trait FileFormat
where
    Self: Format,
    Self::EncodeError: Display,
    Self::DecodeError: Display,
{
    /// Returns the file extension of this [`Format`].
    fn extension(&self) -> String;
}

/// An error that may occur during storage system usage.
#[derive(Debug, thiserror::Error)]
pub enum Error<F>
where
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// An IO error.
    #[error(transparent)]
    File(#[from] std::io::Error),
    /// An encoding error.
    #[error("{0}")]
    Encode(F::EncodeError),
    /// A decoding error.
    #[error("{0}")]
    Decode(F::DecodeError),
}

/// A file-system-based [`System<T, F>`] implementation.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FileSystem(Box<Path>);

impl FileSystem {
    /// Creates a new [`FileSystem`].
    #[inline]
    pub fn new(dir: impl AsRef<Path>) -> Self { Self(dir.as_ref().into()) }

    /// Returns a reference to the storage directory of this [`FileSystem`].
    #[inline]
    #[must_use]
    pub const fn dir(&self) -> &Path { &self.0 }
}

impl Default for FileSystem {
    #[inline]
    fn default() -> Self { Self::new("data") }
}

impl<T, F> System<T, F> for FileSystem
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    type Error = Error<F>;
    type Key = FileKey<T, F>;
    type Val = FileVal<T, F>;

    #[inline]
    fn instance<'s>() -> &'s Self { system() }

    #[inline]
    fn exists(&self, key: Self::Key) -> Result<(Self::Key, bool), Self::Error> {
        key.path().try_exists().map(|b| (key, b)).map_err(Into::into)
    }

    fn read(&self, key: Self::Key) -> Result<Self::Val, Self::Error> {
        let bytes = std::fs::read(key.path())?;
        let data = key.format().decode(&bytes).map_err(Error::Decode)?;

        Ok(key.assign(data))
    }

    fn write(&self, val: Self::Val) -> Result<Self::Key, Self::Error> {
        let bytes = val.key().format().encode(val.data()).map_err(Error::Encode)?;

        val.key().path().parent().map_or(Ok(()), std::fs::create_dir_all)?;
        std::fs::write(val.key().path(), bytes)?;

        Ok(val.key_owned())
    }

    fn remove(&self, key: Self::Key) -> Result<Self::Val, Self::Error> {
        let val = self.read(key)?;

        std::fs::remove_file(val.key().path())?;

        Ok(val)
    }
}

/// A file-system-based [`Key<T, F>`] implementation.
#[allow(clippy::derive_partial_eq_without_eq)] // false positive
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileKey<T, F>(Box<Path>, F, PhantomData<T>)
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display;

impl<T, F> FileKey<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// Creates a new [`FileKey<T, F>`].
    pub fn new(path: impl AsRef<Path>, format: F) -> Self {
        let root = <<Self as Key<T, F>>::System as System<T, F>>::instance().dir();
        let path = root.join(path).with_extension(format.extension());

        Self(path.into_boxed_path(), format, PhantomData)
    }

    /// Creates a new [`FileKey<T, F>`] with a default format value.
    #[inline]
    pub fn new_default(path: impl AsRef<Path>) -> Self
    where
        F: Default,
    {
        Self::new(path, F::default())
    }

    /// Returns a reference to the path of this [`FileKey<T, F>`].
    #[inline]
    pub const fn path(&self) -> &Path { &self.0 }
}

impl<T, F, P: AsRef<Path>> From<P> for FileKey<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: Default + FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    #[inline]
    fn from(value: P) -> Self { Self::new_default(value) }
}

impl<T, F> Key<T, F> for FileKey<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    type System = FileSystem;
    type Val = FileVal<T, F>;

    #[inline]
    fn assign(self, data: T) -> Self::Val { FileVal::new(self, data) }

    #[inline]
    fn format(&self) -> &F { &self.1 }
}

/// A file-system-based [`Val<T, F>`] implementation.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileVal<T, F>(<Self as Val<T, F>>::Key, T)
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display;

impl<T, F> FileVal<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    /// Creates a new [`FileVal<T, F>`].
    pub const fn new(key: <Self as Val<T, F>>::Key, data: T) -> Self { Self(key, data) }

    /// Creates a new [`FileVal<T, F>`] containing default data.
    pub fn new_default(key: <Self as Val<T, F>>::Key) -> Self
    where
        T: Default,
    {
        Self::new(key, T::default())
    }
}

impl<T, F> Val<T, F> for FileVal<T, F>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FileFormat,
    F::EncodeError: Display,
    F::DecodeError: Display,
{
    type System = FileSystem;
    type Key = FileKey<T, F>;

    #[inline]
    fn key(&self) -> &Self::Key { &self.0 }

    #[inline]
    fn key_owned(self) -> Self::Key { self.0 }

    #[inline]
    fn data(&self) -> &T { &self.1 }

    #[inline]
    fn data_mut(&mut self) -> &mut T { &mut self.1 }

    #[inline]
    fn data_owned(self) -> T { self.1 }
}
