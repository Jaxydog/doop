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
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, clap::ValueEnum)]
        pub enum Locale {$(
            $(#[$attribute])*
            #[doc = $translation]
            #[serde(rename = $key)]
            $name,
        )*}

        impl Locale {
            /// A list of every locale.
            pub const LIST: &'static [Self] = &[$(Self::$name),*];

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
            pub fn get(key: impl AsRef<str>) -> Option<Self> {
                match key.as_ref() {
                    $( $key => Some(Self::$name), )*
                    _ => None,
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
