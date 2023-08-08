#![doc = env!("CARGO_PKG_DESCRIPTION")]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

doop_macros::global! {
    /// The directory that stores the bot's localizations.
    static DIRECTORY: Box<Path> = PathBuf::from("lang").into();
    /// The localizer instance.
    static LOCALIZER: Localizer = Localizer::load(Locale::default());
}

/// Installs and configures the localizer instance with the given source path.
///
/// # Panics
///
/// Panics if the localizer or directory has already been initialized.
#[allow(clippy::expect_used)]
pub fn install_into(prefer: Locale, directory: impl AsRef<OsStr>) {
    DIRECTORY
        .set(PathBuf::from(directory.as_ref()).into())
        .expect("the localizer directory has already been initialized");
    LOCALIZER
        .set(Localizer::load(prefer))
        .expect("the localizer instance has already been initialized");
}

/// Installs and configures the localizer instance.
///
/// # Panics
///
/// Panics if the localizer or directory has already been initialized.
#[inline]
pub fn install(prefer: Locale) { crate::install_into(prefer, "lang"); }

/// An [`Error`] that can occur during usage of the storage system.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error during file IO.
    #[error(transparent)]
    File(#[from] std::io::Error),
    /// An error during localization parsing.
    #[error(transparent)]
    Load(#[from] serde_json::Error),
    /// An invalid locale was provided.
    #[error("locale '{0}' is invalid")]
    Locale(Box<str>),
}

/// Represents a localization map for a specific locale.
///
/// This is essentially just a wrapper for a [`HashMap<Box<str>, Box<str>>`], made to be read-only.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct LocalizationMap(HashMap<Box<str>, Box<str>>);

impl LocalizationMap {
    /// Loads the localization map from the file system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the localization map could not be read or decoded.
    pub fn load_from(locale: &str, directory: &Path) -> Result<Self, Error> {
        let path = directory.join(locale).with_extension("json");
        let bytes = std::fs::read(path)?;

        Ok(serde_json::from_slice(&bytes)?)
    }

    /// Loads the localization map from the file system.
    ///
    /// # Errors
    ///
    /// This function will return an error if the localization map could not be read or decoded.
    #[inline]
    pub fn load(locale: &str) -> Result<Self, Error> { Self::load_from(locale, directory()) }

    /// Returns the text assigned to the given key.
    #[inline]
    pub fn get(&self, key: impl AsRef<str>) -> Option<Cow<str>> {
        self.0.get(key.as_ref()).map(|s| Cow::Borrowed(&(**s)))
    }
}

/// Provides an interface for content localization.
///
/// While all localization functionality is possible through this value, it's easier to use the
/// crate's provided macros. See [`localize!`] and [`localize_map!`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Localizer {
    /// The localizer's preferred locale.
    prefer: Locale,
    /// The localizer's loaded localization maps.
    maps: HashMap<Locale, LocalizationMap>,
}

impl Localizer {
    /// Loads all localization maps from the file system.
    #[inline]
    #[must_use]
    pub fn load(prefer: Locale) -> Self {
        let path = directory();
        let maps = Locale::LIST.iter().filter_map(|locale| {
            let map = LocalizationMap::load_from(locale.key(), path).ok()?;

            Some((*locale, map))
        });

        Self { prefer, maps: maps.collect() }
    }

    /// Returns a reference to the preferred [`Locale`] of this [`Localizer`].
    #[inline]
    #[must_use]
    pub const fn preferred_locale(&self) -> &Locale { &self.prefer }

    /// Returns the localized text assigned to the given [`Locale`].
    ///
    /// If the text was not assigned, the key is returned.
    #[inline]
    pub fn localize(&self, locale: Locale, key: impl AsRef<str> + Clone) -> Cow<str> {
        self.maps
            .get(&locale)
            .and_then(|m| m.get(key.clone()))
            .unwrap_or_else(|| key.as_ref().to_string().into())
    }

    /// Returns the localized text assigned to the given [`Locale`], falling back to the preferred
    /// locale.
    ///
    /// If the text was not assigned, the key is returned.
    #[inline]
    pub fn maybe_localize(&self, locale: Locale, key: impl AsRef<str> + Clone) -> Cow<str> {
        self.maps
            .get(&locale)
            .or_else(|| self.maps.get(self.preferred_locale()))
            .and_then(|m| m.get(key.clone()))
            .unwrap_or_else(|| key.as_ref().to_string().into())
    }

    /// Returns the localized text assigned to the preferred locale.
    ///
    /// If the text was not assigned, the key is returned.
    #[inline]
    pub fn preferred_localize(&self, key: impl AsRef<str> + Clone) -> Cow<str> {
        self.localize(*self.preferred_locale(), key)
    }

    /// Returns a map containing all loaded localizations with text assigned to the given key.
    pub fn localization_map(&self, key: impl AsRef<str> + Clone) -> HashMap<String, String> {
        let key = key.as_ref();
        let map = self.maps.iter().filter_map(|(locale, map)| {
            let text = map.get(key)?.into_owned();

            Some((locale.key().to_string(), text))
        });

        map.collect()
    }
}

/// Fetches the given key's assigned text from the [`Localizer`] instance.
///
/// # Examples
///
/// ```
/// localize!(Locale::EnglishUS => "text.test.{}", 12345);
/// localize!(try Locale::German => "text.test.54321");
/// localize!("text.test.69420")
/// ```
#[macro_export]
macro_rules! localize {
    (try $locale:expr => $($args:tt)+) => {
        $crate::localizer().maybe_localize($locale, format!($($args)+))
    };
    ($locale:expr => $($args:tt)+) => {
        $crate::localizer().localize($locale, format!($($args)+))
    };
    ($($args:tt)+) => {
        $crate::localizer().preferred_localize(format!($($args)+))
    };
}

/// Fetches a map of all localizations with text assigned to the given key.
///
/// # Examples
///
/// ```
/// localize_map!("text.test.91021") 
/// ```
#[macro_export]
macro_rules! localize_map {
    ($($args:tt)+) => {
        $crate::localizer().localization_map(format!($($args)+))
    };
}

/// Defines locales.
///
/// # Examples
///
/// ```
/// locales! {
///     /// English, US
///     "en-US" as EnglishUS,
/// }
/// ```
macro_rules! locales {
    {$(
        #[doc = $translation:literal]
        $(#[$attribute:meta])*
        $key:literal as $name:ident
    ),* $(,)?} => {
        /// Discord's (and the bot's) supported locales.
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
        pub enum Locale {$(
            $(#[$attribute])*
            #[doc = $translation]
            #[serde(rename = $key)]
            $name,
        )*}

        impl Locale {
            /// A list of every locale.
            pub const LIST: &[Self] = &[$(Self::$name),*];

            /// Returns the locale's associated localization key.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {$( Self::$name => $key, )*}
            }

            /// Returns the locale associated with the given localization key.
            ///
            /// # Errors
			///
            /// This function will return an error if the given locale does not exist.
            pub fn get(key: impl AsRef<str>) -> Result<Self, Error> {
                match key.as_ref() {
                    $( $key => Ok(Self::$name), )*
                    key => Err(Error::Locale(Box::from(key))),
                }
            }
        }

        impl std::fmt::Display for Locale {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let translated = match self { $( Self::$name => $translation, )* };

                write!(f, "{translated}")
            }
        }
    };
}

locales! {
    /// Bahasa Indonesia
    "id" as Indonesian,
    /// Dansk
    "da" as Danish,
    /// Deutsch
    "de" as German,
    /// English, UK
    "en-GB" as EnglishUK,
    /// English, US
    #[default]
    "en-US" as EnglishUS,
    /// Español
    "es-ES"	as Spanish,
    /// Français
    "fr" as French,
    /// Hrvatski
    "hr" as Croatian,
    /// Italiano
    "it" as Italian,
    /// Lietuviškai
    "lt" as Lithuanian,
    /// Magyar
    "hu" as Hungarian,
    /// Nederlands
    "nl" as Dutch,
    /// Norsk
    "no" as Norwegian,
    /// Polski
    "pl" as Polish,
    /// Português do Brasil
    "pt-BR"	as PortugueseBR,
    /// Română
    "ro" as RomanianRO,
    /// Suomi
    "fi" as Finnish,
    /// Svenska
    "sv-SE"	as Swedish,
    /// Tiếng Việt
    "vi" as Vietnamese,
    /// Türkçe
    "tr" as Turkish,
    /// Čeština
    "cs" as Czech,
    /// Ελληνικά
    "el" as Greek,
    /// български
    "bg" as Bulgarian,
    /// Pусский
    "ru" as Russian,
    /// Українська
    "uk" as Ukrainian,
    /// हिन्दी
    "hi" as Hindi,
    /// ไทย
    "th" as Thai,
    /// 中文
    "zh-CN"	as ChineseCN,
    /// 日本語
    "ja" as Japanese,
    /// 繁體中文
    "zh-TW"	as ChineseTW,
    /// 한국어
    "ko" as Korean,
}
