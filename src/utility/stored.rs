use std::{
    fs::{create_dir_all, remove_file, File},
    io::Write,
    path::PathBuf,
};

use crate::prelude::*;

#[derive(Debug)]
pub struct Stored<'s, T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    dir: &'s str,
    key: &'s str,
    value: T,
}

impl<'s, T> Stored<'s, T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    pub const DIR: &str = "data";
    pub const EXT: &str = "pack";

    pub fn root() -> PathBuf {
        PathBuf::from(Self::DIR)
    }
    pub fn dir_from(dir: &str) -> PathBuf {
        Self::root().join(dir)
    }
    pub fn path_from(dir: &str, key: &str) -> PathBuf {
        Self::dir_from(dir).join(key).with_extension(Self::EXT)
    }

    pub fn new(dir: &'s str, key: &'s str, value: T) -> Result<Self> {
        let data = rmp_serde::to_vec(&value)?;

        create_dir_all(Self::dir_from(dir))?;

        let mut file = File::create(Self::path_from(dir, key))?;
        file.write_all(&data)?;
        file.flush()?;

        Ok(Self { dir, key, value })
    }
    pub fn new_default(dir: &'s str, key: &'s str) -> Result<Self>
    where
        T: Default,
    {
        let value = T::default();
        let data = rmp_serde::to_vec(&value)?;

        create_dir_all(Self::dir_from(dir))?;

        let mut file = File::create(Self::path_from(dir, key))?;
        file.write_all(&data)?;
        file.flush()?;

        Ok(Self { dir, key, value })
    }
    pub fn read(dir: &'s str, key: &'s str) -> Result<Self> {
        let path = Self::path_from(dir, key);
        let file = File::open(path)?;
        let value = rmp_serde::from_read(file)?;

        Ok(Self { dir, key, value })
    }

    pub fn dir(&self) -> PathBuf {
        Self::dir_from(self.dir)
    }
    pub fn path(&self) -> PathBuf {
        Self::path_from(self.dir, self.key)
    }
    pub fn storage_write(&self) -> Result<()> {
        let data = rmp_serde::to_vec(&self.value)?;

        create_dir_all(self.dir())?;

        let mut file = File::create(self.path())?;

        file.write_all(&data)?;
        file.flush().map_err(Into::into)
    }
    pub fn storage_remove(self) -> Result<T> {
        remove_file(self.path())?;

        Ok(self.value)
    }
}

impl<T> Deref for Stored<'_, T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Stored<'_, T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
