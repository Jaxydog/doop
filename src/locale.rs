use std::borrow::Cow;
use std::collections::HashMap;

use serde::Deserialize;

use crate::extend::IteratorExt;
use crate::utility::Result;

/// Represents a localized map for a locale.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LocalizedMap(HashMap<Box<str>, Box<str>>);

impl LocalizedMap {
    /// The directory that contains all language definitions files.
    pub const DIR: &str = "lang";

    /// Returns the text assigned to the provided key.
    #[inline]
    pub fn get(&self, key: impl AsRef<str>) -> Option<Cow<str>> {
        self.0.get(key.as_ref()).map(|s| Cow::Borrowed(&(**s)))
    }

    /// Reads the localized map from the file system.
    pub fn read(locale: &str) -> Result<Self> {
        let path = format!("{}/{locale}.json", Self::DIR);
        let bytes = std::fs::read(path)?;

        Ok(serde_json::from_slice(&bytes)?)
    }
}

/// Provides an interface for the localization of bot content.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Localizer(HashMap<Box<str>, LocalizedMap>);

impl Localizer {
    /// The default locale for bot localizations.
    pub const DEFAULT: &str = "en-US";
    /// Contains a list of valid Discord locales.
    pub const LOCALES: [&'static str; 31] = [
        "id", "da", "de", "en-GB", "en-US", "es-ES", "fr", "hr", "it", "lt", "hu", "nl", "no",
        "pl", "pt-BR", "ro", "fi", "sv-SE", "vi", "tr", "cs", "el", "bg", "ru", "uk", "hi", "th",
        "zh-CN", "ja", "zh-TW", "ko",
    ];

    /// Loads the bot's localization maps from the file system.
    #[must_use]
    pub fn load() -> Self {
        let locales = Self::LOCALES.into_iter().try_filter_map(|locale| {
            let map = LocalizedMap::read(locale)?;

            Ok::<_, anyhow::Error>((locale.into(), map))
        });

        Self(locales.collect())
    }

    /// Returns the localization assigned to the key in the given locale.
    #[must_use]
    pub fn get(&self, locale: &str, key: impl AsRef<str> + Clone) -> Cow<str> {
        let text = self.0.get(locale).and_then(|m| m.get(key.clone()));

        text.unwrap_or_else(|| key.as_ref().to_owned().into())
    }

    /// Returns the translation assigned to the provided key in the given
    /// locale, defaulting to the default translation locale.
    #[inline]
    #[must_use]
    pub fn get_or_default(&self, locale: Option<&str>, key: impl AsRef<str> + Clone) -> Cow<str> {
        self.get(locale.unwrap_or(Self::DEFAULT), key)
    }

    /// Returns a map of loaded translations assigned to the provided key.
    #[must_use]
    pub fn get_map(&self, key: impl AsRef<str> + Clone) -> HashMap<String, String> {
        let map = self.0.iter().filter_map(|(locale, map)| {
            let text = map.get(key.clone())?.into_owned();

            Some((locale.to_string(), text))
        });

        map.collect()
    }
}

crate::global! {{
    /// Returns the bot's localizer instance.
    [LOCALIZER] fn localizer() -> Localizer { Localizer::load }
}}

/// Fetches the requested (or default) localization for the given key.
///
/// # Examples
/// ```
/// localize!("en-US" => "text.{}.title", State::NAME);
/// localize!("text.{}.title", State::NAME);
/// ```
#[macro_export]
macro_rules! localize {
    ($locale:expr => $($args:tt)+) => {
        $crate::locale::localizer().get_or_default($locale, format!($($args)+).as_str())
    };
    ($($args:tt)+) => {
        $crate::locale::localizer().get_or_default(None, format!($($args)+).as_str())
    };
}

/// Returns all possible localizations for the given key.
///
/// # Examples
/// ```
/// locale_map!("command.{}.name", State::NAME);
/// ```
#[macro_export]
macro_rules! locale_map {
    ($($args:tt)+) => {
        $crate::locale::localizer().get_map(format!($($args)+).as_str())
    };
}
