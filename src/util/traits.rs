use anyhow::bail;
use doop_localizer::{localizer, Locale};
use time::ext::NumericalDuration;
use time::macros::datetime;
use time::OffsetDateTime;
use twilight_cache_inmemory::model::{CachedGuild, CachedMember};
use twilight_model::channel::message::ReactionType;
use twilight_model::guild::{Guild, Member, PartialGuild, PartialMember};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, CurrentUserGuild, User};
use twilight_util::builder::embed::image_source::ImageSourceUrlError;
use twilight_util::builder::embed::ImageSource;

use super::{CDN_URL, TWEMOJI_URL};

/// Provides a method that returns the implementing type's creation date.
pub trait CreatedAt {
    /// Returns the creation date of this value.
    fn created_at(&self) -> OffsetDateTime;
}

impl<T> CreatedAt for Id<T> {
    #[inline]
    fn created_at(&self) -> OffsetDateTime {
        const DISCORD_EPOCH: OffsetDateTime = datetime!(2015-01-01 00:00:00 UTC);

        #[allow(clippy::cast_possible_wrap)]
        let milliseconds = (self.get() >> 22) as i64;

        DISCORD_EPOCH.saturating_add(milliseconds.milliseconds())
    }
}

/// Provides a method that returns the implementing type's associated icon.
pub trait GetIcon {
    /// The type returned if an icon cannot be created.
    type Error;

    /// Returns the icon of this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the icon could not be created.
    fn get_icon(&self) -> Result<ImageSource, Self::Error>;
}

impl GetIcon for CachedGuild {
    type Error = anyhow::Error;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon() else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id());

        Ok(ImageSource::url(url)?)
    }
}

impl GetIcon for CurrentUser {
    type Error = ImageSourceUrlError;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        ImageSource::url(self.avatar.map_or_else(
            || format!("{CDN_URL}/embed/avatars/{}.png", self.id.get() % 5),
            |hash| {
                let ext = if hash.is_animated() { "gif" } else { "png" };

                format!("{CDN_URL}/avatars/{}/{hash}.{ext}", self.id)
            },
        ))
    }
}

impl GetIcon for CurrentUserGuild {
    type Error = anyhow::Error;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

impl GetIcon for Guild {
    type Error = anyhow::Error;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

impl GetIcon for PartialGuild {
    type Error = anyhow::Error;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

impl GetIcon for ReactionType {
    type Error = ImageSourceUrlError;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        let url = match self {
            Self::Custom { animated, id, .. } => {
                let ext = if *animated { "gif" } else { "png" };

                format!("{CDN_URL}/emojis/{id}.{ext}")
            }
            Self::Unicode { name } => {
                let id = name.chars().map(|n| format!("{:x}", n as u32));

                format!("{TWEMOJI_URL}/72x72/{}.png", id.collect::<Vec<_>>().join("-"))
            }
        };

        ImageSource::url(url)
    }
}

impl GetIcon for User {
    type Error = ImageSourceUrlError;

    fn get_icon(&self) -> Result<ImageSource, Self::Error> {
        ImageSource::url(self.avatar.map_or_else(
            || format!("{CDN_URL}/embed/avatars/{}.png", self.id.get() % 5),
            |hash| {
                let ext = if hash.is_animated() { "gif" } else { "png" };

                format!("{CDN_URL}/avatars/{}/{hash}.{ext}", self.id)
            },
        ))
    }
}

impl<T: GetIcon> GetIcon for &T {
    type Error = T::Error;

    #[inline]
    fn get_icon(&self) -> Result<ImageSource, Self::Error> { T::get_icon(self) }
}

/// Provides a method that returns the implementing type's associated icon.
pub trait GetIconWith {
    /// The arguments provided to the `get_icon_with` method.
    type Arguments;
    /// The type returned if an icon cannot be created.
    type Error;

    /// Returns the icon of this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the icon could not be created.
    fn get_icon_with(&self, _: Self::Arguments) -> Result<ImageSource, Self::Error>;
}

impl GetIconWith for CachedMember {
    type Arguments = Id<GuildMarker>;
    type Error = anyhow::Error;

    fn get_icon_with(&self, guild_id: Self::Arguments) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.avatar() else {
            bail!("missing avatar hash")
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/guilds/{guild_id}/users/{}/{hash}.{ext}", self.user_id());

        Ok(ImageSource::url(url)?)
    }
}

impl GetIconWith for Member {
    type Arguments = Id<GuildMarker>;
    type Error = ImageSourceUrlError;

    fn get_icon_with(&self, guild_id: Self::Arguments) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.avatar else {
            return self.user.get_icon();
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/guilds/{guild_id}/users/{}/{hash}.{ext}", self.user.id);

        ImageSource::url(url)
    }
}

impl GetIconWith for PartialMember {
    type Arguments = Id<GuildMarker>;
    type Error = anyhow::Error;

    fn get_icon_with(&self, guild_id: Self::Arguments) -> Result<ImageSource, Self::Error> {
        let Some(ref user) = self.user else {
            bail!("missing user data for member");
        };
        let Some(hash) = self.avatar else {
            return Ok(user.get_icon()?);
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/guilds/{guild_id}/users/{}/{hash}.{ext}", user.id);

        Ok(ImageSource::url(url)?)
    }
}

impl<T: GetIcon> GetIconWith for T {
    type Arguments = ();
    type Error = <T as GetIcon>::Error;

    #[inline]
    fn get_icon_with(&self, _: Self::Arguments) -> Result<ImageSource, Self::Error> {
        self.get_icon()
    }
}

/// Provides a method that returns the implementing type's associated locale.
pub trait Localized {
    /// Returns the preferred locale of this value.
    fn preferred_locale(&self) -> Option<Locale>;

    /// Returns the preferred locale of this value, or the default value if `None`.
    #[inline]
    fn locale(&self) -> Locale {
        self.preferred_locale().unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl Localized for CachedGuild {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { Locale::get(self.preferred_locale()).ok() }
}

impl Localized for CurrentUser {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { Locale::get(self.locale.as_ref()?).ok() }
}

impl Localized for Guild {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { Locale::get(&self.preferred_locale).ok() }
}

impl Localized for PartialGuild {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { Locale::get(&self.preferred_locale).ok() }
}

impl Localized for User {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { Locale::get(self.locale.as_ref()?).ok() }
}

impl Localized for Member {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { self.user.preferred_locale() }
}

impl Localized for PartialMember {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { self.user.as_ref()?.preferred_locale() }
}

impl<T: Localized> Localized for &T {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> { <T as Localized>::preferred_locale(self) }
}

impl<T: Localized> Localized for Option<T> {
    #[inline]
    fn preferred_locale(&self) -> Option<Locale> {
        self.as_ref().and_then(Localized::preferred_locale)
    }
}
