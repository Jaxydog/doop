use std::fmt::Display;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::Result;

/// Defines a custom data-storing identifier for use in components and modals.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataId {
    /// The base identifier.
    base: Box<str>,
    /// The identifier's sub-identifier.
    path: Option<Box<str>>,
    /// The data stored within the identifier.
    data: Vec<Box<str>>,
}

impl DataId {
    /// The maximum number of allowed bytes within a stringified identifier.
    pub const MAX_LEN: usize = 100;

    /// Creates a new custom identifier containing no data.
    pub fn new_empty(base: impl Into<String>, path: Option<impl Into<String>>) -> Result<Self> {
        let base = base.into().into_boxed_str();
        let path = path.map(|path| path.into().into_boxed_str());
        let this = Self { base, path, data: vec![] };

        if this.to_string().len() <= Self::MAX_LEN {
            Ok(this)
        } else {
            Err(anyhow!("max size exceeded (> {} bytes)", Self::MAX_LEN))
        }
    }

    /// Creates a new custom identifier containing the provided data.
    pub fn new(
        base: impl Into<String>,
        path: Option<impl Into<String>>,
        data: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self> {
        let mut this = Self::new_empty(base, path)?;

        data.into_iter().try_for_each(|d| this.with(d))?;

        Ok(this)
    }

    /// Returns the custom identifier's base identifier.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> &str {
        &self.base
    }

    /// Returns the custom identifier's path identifier.
    #[inline]
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the custom identifier's data content.
    #[inline]
    #[must_use]
    pub fn data(&self) -> &[Box<str>] {
        &self.data
    }

    /// Attempts to insert the provided data into the custom identifier.
    pub fn with(&mut self, data: impl Into<String>) -> Result {
        let data = data.into();

        if self.to_string().len() + data.len() <= Self::MAX_LEN {
            self.data.push(data.into_boxed_str());

            Ok(())
        } else {
            Err(anyhow!("max size exceeded (> {} bytes)", Self::MAX_LEN))
        }
    }

    /// Attempts to insert the provided data into the custom identifier.
    ///
    /// This allows for chaining, unlike `with`
    pub fn join(mut self, data: impl Into<String>) -> Result<Self> {
        self.with(data)?;

        Ok(self)
    }
}

impl TryFrom<&str> for DataId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split(';').take(2);

        let Some(mut ids) = parts.next().map(|s| s.split(':')) else {
            return Err(anyhow!("missing identifiers"));
        };
        let Some(base) = ids.next() else {
            return Err(anyhow!("missing base identifier"));
        };

        Self::new(base, ids.next(), parts.next().unwrap_or("").split(','))
    }
}

impl TryFrom<String> for DataId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<DataId> for String {
    fn from(value: DataId) -> Self {
        value.to_string()
    }
}

impl Display for DataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { base, path, data } = self;

        write!(f, "{base}")?;

        if let Some(path) = path {
            write!(f, ":{path}")?;
        }
        if !data.is_empty() {
            write!(f, ";{}", data.join(","))?;
        }

        Ok(())
    }
}
