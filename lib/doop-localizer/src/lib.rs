//! Implements a content localizer for the Doop Discord bot.
#![deny(clippy::expect_used, unsafe_code, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::todo, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock, RwLockReadGuard};

use doop_logger::{info, warn};

pub use crate::locale::*;

mod locale;

/// The global localizer.
static LOCALIZER: OnceLock<RwLock<Localizer>> = OnceLock::new();

/// Returns a reference to the global localizer.
///
/// # Panics
///
/// Panics if the localizer has not been initialized or is poisoned.
#[allow(clippy::expect_used, clippy::unwrap_used)]
pub fn localizer() -> RwLockReadGuard<'static, Localizer> {
    LOCALIZER.get().expect("the localizer has not been initialized").read().unwrap()
}

/// Initializes the global localizer.
///
/// # Panics
///
/// Panics if the localizer has already been initialized.
#[allow(clippy::expect_used)]
pub fn install(prefer: Locale, dir: impl AsRef<Path>) {
    let localizer = Localizer::new(prefer, dir);
    let locales = localizer.content.keys().map(|l| l.key()).collect::<Vec<_>>().join(", ");

    info!("loaded localizations: [{locales}]",).ok();

    LOCALIZER.set(RwLock::new(localizer)).expect("the localizer has already been initialized");
}

/// Reloads the global localizer.
///
/// # Panics
///
/// Panics if the localizer has not been initialized or is poisoned.
#[allow(clippy::expect_used, clippy::unwrap_used)]
pub fn reload(prefer: Locale, dir: impl AsRef<Path>) {
    let localizer = Localizer::new(prefer, dir);
    let locales = localizer.content.keys().map(|l| l.key()).collect::<Vec<_>>().join(", ");

    info!("loaded localizations: [{locales}]",).ok();

    *LOCALIZER.get().expect("the localizer has not been initialized").write().unwrap() = localizer;
}

/// Provides an interface for content localization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Localizer {
    /// The localizer's preferred locale.
    prefer: Locale,
    /// The localizer's internal locale-content map.
    content: HashMap<Locale, HashMap<Box<str>, Box<str>>>,
}

impl Localizer {
    /// Creates a new [`Localizer`], attempting to load content maps from the given directory.
    #[must_use]
    pub fn new(prefer: Locale, dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        let content = Locale::LIST.iter().filter_map(|locale| {
            let path = dir.join(locale.key()).with_extension("json");
            let bytes = std::fs::read(path).ok()?;

            match serde_json::from_slice(&bytes) {
                Ok(value) => Some((*locale, value)),
                Err(error) => {
                    warn!("failed to load localization map '{}' - {error}", locale.key()).ok();
                    None
                }
            }
        });

        Self { prefer, content: content.collect() }
    }

    /// Returns the preferred locale of this [`Localizer`].
    #[must_use]
    pub const fn preferred_locale(&self) -> &Locale {
        &self.prefer
    }

    /// Returns the text assigned to the provided key in the preferred locale.
    ///
    /// If the locale is missing or the key is unassigned, the key is returned.
    pub fn localize_preferred(&self, key: impl AsRef<str>) -> Box<str> {
        let key = key.as_ref();
        let text = self.content.get(self.preferred_locale()).and_then(|map| map.get(key));

        text.map_or_else(|| key.into(), Clone::clone)
    }

    /// Returns the text assigned to the provided key in the given locale.
    ///
    /// If the locale is missing or the key is unassigned, the key is returned.
    pub fn localize(&self, locale: Locale, key: impl AsRef<str>) -> Box<str> {
        let key = key.as_ref();
        let text = self.content.get(&locale).and_then(|map| map.get(key));

        text.map_or_else(|| key.into(), Clone::clone)
    }

    /// Returns the text assigned to the provided key in the given locale or the preferred locale.
    ///
    /// If either locale is missing or the key is unassigned, the key is returned.
    pub fn maybe_localize(&self, locale: Locale, key: impl AsRef<str>) -> Box<str> {
        let key = key.as_ref();
        let map = self.content.get(&locale).or_else(|| self.content.get(self.preferred_locale()));
        let text = map.and_then(|map| map.get(key));

        text.map_or_else(|| key.into(), Clone::clone)
    }

    /// Returns a map containing all loaded locales that contain the given key and their assigned
    /// translations.
    pub fn localizations(&self, key: impl AsRef<str>) -> HashMap<String, String> {
        let key = key.as_ref();
        let map = self.content.iter().filter_map(|(locale, map)| {
            let text = map.get(key)?.to_string();
            let key = locale.key().to_string();

            Some((key, text))
        });

        map.collect()
    }
}

/// Fetches the given key's assigned text from the [`Localizer`].
///
/// # Examples
///
/// ```
/// // Returns the text in the default locale.
/// localize!("command.ping.name");
/// // Returns the text in German.
/// localize!(in Locale::German, "command.ping.name");
/// // Returns the text in German, or in the default locale if German is not loaded.
/// localize!(try in Locale::German, "command.ping.name");
///
/// // Returns a map of the text in all containing locales.
/// localize!(in *, "command.ping.name");
/// ```
#[macro_export]
macro_rules! localize {
    ($(try)? in *, $($args:tt)+) => {
        $crate::localizer().localizations(format!($($args)+))
    };
    (try in $locale:expr, $($args:tt)+) => {
        $crate::localizer().maybe_localize($locale, format!($($args)+))
    };
    (in $locale:expr, $($args:tt)+) => {
        $crate::localizer().localize($locale, format!($($args)+))
    };
    ($($args:tt)+) => {
        $crate::localizer().localize_preferred(format!($($args)+))
    };
}
