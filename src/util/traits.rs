use anyhow::bail;
use doop_localizer::{localizer, Locale};
use time::ext::NumericalDuration;
use time::macros::datetime;
use time::OffsetDateTime;
use twilight_cache_inmemory::model::{CachedGuild, CachedMember};
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::ReactionType;
use twilight_model::guild::{Guild, Member, PartialMember};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, CurrentUserGuild, User};
use twilight_util::builder::embed::image_source::ImageSourceUrlError;
use twilight_util::builder::embed::ImageSource;

use crate::util::{CDN_URL, TWEMOJI_URL};

/// Provides a method that returns the implementing type's creation date.
pub trait Created {
    /// Returns the creation time and date of this value.
    fn created_at(&self) -> OffsetDateTime;
}

impl<T: Created> Created for &T {
    #[inline]
    fn created_at(&self) -> OffsetDateTime {
        <T as Created>::created_at(self)
    }
}

impl<T> Created for Id<T> {
    fn created_at(&self) -> OffsetDateTime {
        const DISCORD_EPOCH: OffsetDateTime = datetime!(2015-01-01 00:00:00 UTC);

        #[allow(clippy::cast_possible_wrap)]
        let milliseconds = (self.get() >> 22) as i64;

        DISCORD_EPOCH.saturating_add(milliseconds.milliseconds())
    }
}

/// Provides a method that returns the implementing type's associated image.
pub trait IntoImageSource {
    /// The type returned if an icon cannot be created.
    type Error;

    /// Returns the image source associated with this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the icon could not be created.
    fn into_image_source(self) -> Result<ImageSource, Self::Error>;
}

impl IntoImageSource for &CachedGuild {
    type Error = anyhow::Error;

    fn into_image_source(self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon() else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id());

        Ok(ImageSource::url(url)?)
    }
}

impl IntoImageSource for &CurrentUser {
    type Error = anyhow::Error;

    fn into_image_source(self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.avatar else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/avatars/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

impl IntoImageSource for &CurrentUserGuild {
    type Error = anyhow::Error;

    fn into_image_source(self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

impl IntoImageSource for &Guild {
    type Error = anyhow::Error;

    fn into_image_source(self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.icon else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/icons/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

impl IntoImageSource for &ReactionType {
    type Error = ImageSourceUrlError;

    fn into_image_source(self) -> Result<ImageSource, Self::Error> {
        ImageSource::url(match self {
            ReactionType::Custom { animated, id, .. } => {
                let ext = if *animated { "gif" } else { "png" };

                format!("{CDN_URL}/emojis/{id}.{ext}")
            }
            ReactionType::Unicode { name } => {
                let id = name.chars().map(|c| format!("{:x}", c as u32)).collect::<Vec<_>>();

                format!("{TWEMOJI_URL}/72x72/{}.png", id.join("-"))
            }
        })
    }
}

impl IntoImageSource for &User {
    type Error = anyhow::Error;

    fn into_image_source(self) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.avatar else {
            bail!("the guild's icon is not set");
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/avatars/{}/{hash}.{ext}", self.id);

        Ok(ImageSource::url(url)?)
    }
}

/// Provides a method that returns the implementing type's associated image.
pub trait IntoImageSourceWith {
    /// The arguments provided to the `into_image_source` method.
    type Arguments;
    /// The type returned if an icon cannot be created.
    type Error;

    /// Returns the image source associated with this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the icon could not be created.
    fn into_image_source(self, _: Self::Arguments) -> Result<ImageSource, Self::Error>;
}

impl IntoImageSourceWith for &CachedMember {
    type Arguments = Id<GuildMarker>;
    type Error = anyhow::Error;

    fn into_image_source(self, guild_id: Self::Arguments) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.avatar() else {
            bail!("missing avatar hash")
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url =
            format!("{CDN_URL}/guilds/{guild_id}/users/{}/avatars/{hash}.{ext}", self.user_id());

        Ok(ImageSource::url(url)?)
    }
}

impl IntoImageSourceWith for &Member {
    type Arguments = Id<GuildMarker>;
    type Error = anyhow::Error;

    fn into_image_source(self, guild_id: Self::Arguments) -> Result<ImageSource, Self::Error> {
        let Some(hash) = self.avatar else {
            return self.user.into_image_source();
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url =
            format!("{CDN_URL}/guilds/{guild_id}/users/{}/avatars/{hash}.{ext}", self.user.id);

        Ok(ImageSource::url(url)?)
    }
}

impl IntoImageSourceWith for &PartialMember {
    type Arguments = Id<GuildMarker>;
    type Error = anyhow::Error;

    fn into_image_source(self, guild_id: Self::Arguments) -> Result<ImageSource, Self::Error> {
        let Some(ref user) = self.user else {
            bail!("missing user data for member");
        };
        let Some(hash) = self.avatar else {
            return user.into_image_source();
        };

        let ext = if hash.is_animated() { "gif" } else { "png" };
        let url = format!("{CDN_URL}/guilds/{guild_id}/users/{}/avatars/{hash}.{ext}", user.id);

        Ok(ImageSource::url(url)?)
    }
}

/// Specifies that the implementing type prefers a given locale.
pub trait PreferLocale {
    /// The preferred locale of this type.
    fn preferred_locale(&self) -> Locale;
}

impl<T: PreferLocale> PreferLocale for &T {
    #[inline]
    fn preferred_locale(&self) -> Locale {
        <T as PreferLocale>::preferred_locale(self)
    }
}

impl<T: PreferLocale> PreferLocale for Option<T> {
    #[inline]
    fn preferred_locale(&self) -> Locale {
        self.as_ref()
            .map_or_else(|| *localizer().preferred_locale(), PreferLocale::preferred_locale)
    }
}

impl PreferLocale for CachedGuild {
    fn preferred_locale(&self) -> Locale {
        Locale::get(self.preferred_locale()).unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl PreferLocale for CurrentUser {
    fn preferred_locale(&self) -> Locale {
        self.locale
            .as_deref()
            .and_then(Locale::get)
            .unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl PreferLocale for Guild {
    fn preferred_locale(&self) -> Locale {
        Locale::get(&self.preferred_locale).unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl PreferLocale for Interaction {
    #[inline]
    fn preferred_locale(&self) -> Locale {
        self.locale
            .as_deref()
            .and_then(Locale::get)
            .unwrap_or_else(|| *localizer().preferred_locale())
    }
}

impl PreferLocale for Member {
    #[inline]
    fn preferred_locale(&self) -> Locale {
        self.user.preferred_locale()
    }
}

impl PreferLocale for PartialMember {
    #[inline]
    fn preferred_locale(&self) -> Locale {
        self.user.preferred_locale()
    }
}

impl PreferLocale for User {
    fn preferred_locale(&self) -> Locale {
        self.locale
            .as_deref()
            .and_then(Locale::get)
            .unwrap_or_else(|| *localizer().preferred_locale())
    }
}
