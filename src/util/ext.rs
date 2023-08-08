use anyhow::{anyhow, bail};
use doop_localizer::{localizer, Locale};
use time::ext::NumericalDuration;
use time::macros::datetime;
use time::OffsetDateTime;
use twilight_cache_inmemory::model::CachedGuild;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::ReactionType;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::{Guild, Member, PartialMember};
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, User};
use twilight_util::builder::embed::image_source::ImageSourceUrlError;
use twilight_util::builder::embed::{EmbedAuthorBuilder, ImageSource};

use super::{Result, BRANDING};

/// Discord content delivery network endpoint base URL.
pub const CDN_BASE: &str = "https://cdn.discordapp.com";
/// Discord's emoji repository's base URL.
pub const UNICODE_BASE: &str = "https://raw.githubusercontent.com/discord/twemoji/master/assets";

/// Provides extensions for [`EmbedAuthor`]s and [`EmbedAuthorBuilder`]s.
pub trait EmbedAuthorExt<T>: Sized {
    /// The type that will be returned if the embed author could not be created.
    type Error;

    /// Creates a new [`EmbedAuthor`] from the given value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the author could not be created.
    fn create(value: &T) -> Result<Self, Self::Error>;
}

impl<T: UserExt> EmbedAuthorExt<T> for EmbedAuthorBuilder {
    type Error = ImageSourceUrlError;

    #[inline]
    fn create(value: &T) -> Result<Self, Self::Error> {
        Ok(Self::new(value.tag()).icon_url(value.icon()?))
    }
}

impl<T> EmbedAuthorExt<T> for EmbedAuthor
where
    EmbedAuthorBuilder: EmbedAuthorExt<T>,
{
    type Error = <EmbedAuthorBuilder as EmbedAuthorExt<T>>::Error;

    #[inline]
    fn create(value: &T) -> Result<Self, Self::Error> {
        EmbedAuthorBuilder::create(value).map(EmbedAuthorBuilder::build)
    }
}

/// Provides extensions for values with associated creation dates.
pub trait CreatedAtExt {
    /// Returns the creation date of this value.
    fn created_at(&self) -> OffsetDateTime;
}

impl<T> CreatedAtExt for Id<T> {
    #[inline]
    fn created_at(&self) -> OffsetDateTime {
        const DISCORD_EPOCH: OffsetDateTime = datetime!(2015-01-01 00:00:00 UTC);

        #[allow(clippy::cast_possible_wrap)]
        let milliseconds = (self.get() >> 22) as i64;

        DISCORD_EPOCH.saturating_add(milliseconds.milliseconds())
    }
}

/// Provides extensions for [`Interaction`]s.
pub trait InteractionExt {
    /// Returns the internal label of this [`Interaction`].
    fn label(&self) -> String;
}

impl InteractionExt for InteractionCreate {
    fn label(&self) -> String {
        self.author_id().map_or_else(
            || format!("<{:?} #{}>", self.kind, self.id),
            |id| format!("<{:?} #{} @{id}>", self.kind, self.id),
        )
    }
}

impl InteractionExt for Interaction {
    fn label(&self) -> String {
        self.author_id().map_or_else(
            || format!("<{:?} #{}>", self.kind, self.id),
            |id| format!("<{:?} #{} @{id}>", self.kind, self.id),
        )
    }
}

/// Provides extensions for values with associated locales.
pub trait LocalizedExt {
    /// Returns the preferred locale of this value.
    fn locale(&self) -> Locale;
}

impl LocalizedExt for CachedGuild {
    #[inline]
    fn locale(&self) -> Locale {
        Locale::get(self.preferred_locale()).unwrap_or_else(|_| *localizer().preferred_locale())
    }
}

impl LocalizedExt for CurrentUser {
    #[inline]
    fn locale(&self) -> Locale {
        self.locale
            .as_deref()
            .and_then(|s| Locale::get(s).ok())
            .unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl LocalizedExt for Guild {
    #[inline]
    fn locale(&self) -> Locale {
        Locale::get(&self.preferred_locale).unwrap_or_else(|_| *localizer().preferred_locale())
    }
}

impl LocalizedExt for Member {
    #[inline]
    fn locale(&self) -> Locale { self.user.locale() }
}

impl LocalizedExt for PartialMember {
    #[inline]
    fn locale(&self) -> Locale { self.user.locale() }
}

impl LocalizedExt for User {
    #[inline]
    fn locale(&self) -> Locale {
        self.locale
            .as_deref()
            .and_then(|s| Locale::get(s).ok())
            .unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl<T: LocalizedExt> LocalizedExt for &T {
    #[inline]
    fn locale(&self) -> Locale { <T as LocalizedExt>::locale(self) }
}

impl<T: LocalizedExt> LocalizedExt for Option<T> {
    #[inline]
    fn locale(&self) -> Locale {
        self.as_ref()
            .map_or_else(|| *localizer().preferred_locale(), LocalizedExt::locale)
    }
}

/// Provides extensions for [`ReactionType`]s.
pub trait ReactionTypeExt: Sized {
    /// Returns an image source of the emoji's image.
    ///
    /// # Errors
    ///
    /// This function will return an error if the source has an invalid URL.
    fn icon(&self) -> Result<ImageSource, ImageSourceUrlError>;
}

impl ReactionTypeExt for ReactionType {
    fn icon(&self) -> Result<ImageSource, ImageSourceUrlError> {
        let url = match self {
            Self::Custom { animated, id, .. } => {
                let ext = if *animated { "gif" } else { "png" };

                format!("{CDN_BASE}/emojis/{id}.{ext}")
            }
            Self::Unicode { name } => {
                let id: Vec<_> = name.chars().map(|n| format!("{:x}", n as u32)).collect();

                format!("{UNICODE_BASE}/72x72/{}.png", id.join("-"))
            }
        };

        ImageSource::url(url)
    }
}

/// Provides a `parse` function for [`ReactionType`]s.
pub trait ReactionTypeParseExt<T>: Sized {
    /// Creates a new [`ReactionType`] from the given [`str`] reference.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be parsed.
    fn parse(value: T) -> Result<Self>;
}

impl ReactionTypeParseExt<String> for ReactionType {
    #[inline]
    fn parse(value: String) -> Result<Self> { Self::parse(&(*value)) }
}

impl ReactionTypeParseExt<&String> for ReactionType {
    #[inline]
    fn parse(value: &String) -> Result<Self> { Self::parse(&(**value)) }
}

impl ReactionTypeParseExt<Box<str>> for ReactionType {
    #[inline]
    fn parse(value: Box<str>) -> Result<Self> { Self::parse(&(*value)) }
}

impl ReactionTypeParseExt<&Box<str>> for ReactionType {
    #[inline]
    fn parse(value: &Box<str>) -> Result<Self> { Self::parse(&(**value)) }
}

impl ReactionTypeParseExt<&str> for ReactionType {
    fn parse(value: &str) -> Result<Self> {
        match value.chars().count() {
            0 => bail!("expected a non-empty string"),
            #[allow(clippy::unwrap_used)] // guaranteed to exist
            1 => return Self::parse(value.chars().next().unwrap()),
            _ => {}
        }

        if !value.starts_with('<') {
            return Ok(Self::Unicode { name: value.to_string() });
        }
        if !value.ends_with('>') {
            bail!("invalid emoji formatting");
        }

        let inner = value.trim_matches(&['<', '>'] as &[char]);
        let mut split = inner.split(':');

        let animated = if let Some(slice) = split.next() {
            slice == "a"
        } else {
            bail!("invalid emoji formatting");
        };
        let Some(name) = split.next().map(ToString::to_string) else {
            bail!("invalid emoji formatting");
        };
        let id = if let Some(id) = split.next() {
            Id::new_checked(id.parse()?).ok_or_else(|| anyhow!("expected a non-zero identifier"))?
        } else {
            bail!("invalid emoji formatting");
        };

        Ok(Self::Custom { animated, id, name: Some(name) })
    }
}

impl ReactionTypeParseExt<char> for ReactionType {
    #[inline]
    fn parse(value: char) -> Result<Self> { Ok(Self::Unicode { name: value.to_string() }) }
}

/// Provides extensions for [`str`]s.
pub trait StrExt {
    /// Collapses common escape sequences.
    fn collapse(&self) -> String;
}

impl StrExt for &str {
    #[inline]
    fn collapse(&self) -> String {
        self.replace(r"\n", "\n").replace(r"\t", "\t").replace(r"\r", "\r")
    }
}

/// Provides extensions for [`User`]s.
pub trait UserExt {
    /// Returns the tag of this [`User`], either in the current format (`@username`) or old format
    /// (`Username#1234`).
    fn tag(&self) -> String;

    /// Returns the accent color of this [`User`], or the bot's default brand color.
    fn color(&self) -> u32;

    /// Returns the user's avatar image source, defaulting to a standard avatar.
    ///
    /// # Errors
    ///
    /// This function will return an error if the user's avatar hash is invalid.
    fn icon(&self) -> Result<ImageSource, ImageSourceUrlError>;
}

impl UserExt for CurrentUser {
    fn tag(&self) -> String {
        if self.discriminator == 0 {
            format!("@{}", self.name)
        } else {
            format!("{}#{}", self.name, self.discriminator())
        }
    }

    #[inline]
    fn color(&self) -> u32 { self.accent_color.unwrap_or(BRANDING) }

    fn icon(&self) -> Result<ImageSource, ImageSourceUrlError> {
        let url = self.avatar.map_or_else(
            || format!("{CDN_BASE}/embed/avatars/{}.png", self.id.get() % 5),
            |hash| format!("{CDN_BASE}/avatars/{}/{hash}.png", self.id),
        );

        ImageSource::url(url)
    }
}

impl UserExt for Member {
    fn tag(&self) -> String {
        if self.user.discriminator == 0 {
            format!("@{}", self.user.name)
        } else {
            format!("{}#{}", self.user.name, self.user.discriminator())
        }
    }

    #[inline]
    fn color(&self) -> u32 { self.user.color() }

    fn icon(&self) -> Result<ImageSource, ImageSourceUrlError> {
        let url = self.avatar.or(self.user.avatar).map_or_else(
            || format!("{CDN_BASE}/embed/avatars/{}.png", self.user.id.get() % 5),
            |hash| format!("{CDN_BASE}/avatars/{}/{hash}.png", self.user.id),
        );

        ImageSource::url(url)
    }
}

impl UserExt for User {
    fn tag(&self) -> String {
        if self.discriminator == 0 {
            format!("@{}", self.name)
        } else {
            format!("{}#{}", self.name, self.discriminator())
        }
    }

    #[inline]
    fn color(&self) -> u32 { self.accent_color.unwrap_or(BRANDING) }

    fn icon(&self) -> Result<ImageSource, ImageSourceUrlError> {
        let url = self.avatar.map_or_else(
            || format!("{CDN_BASE}/embed/avatars/{}.png", self.id.get() % 5),
            |hash| format!("{CDN_BASE}/avatars/{}/{hash}.png", self.id),
        );

        ImageSource::url(url)
    }
}
