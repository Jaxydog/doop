use std::borrow::Cow;

use anyhow::{anyhow, bail};
use twilight_cache_inmemory::model::CachedGuild;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::ReactionType;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::{Guild, Member, PartialMember};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, User};
use twilight_util::builder::embed::EmbedAuthorBuilder;

use crate::util::traits::{IntoImageSource, IntoImageSourceWith};
use crate::util::BRANDING;

/// Provides type extensions for [`EmbedAuthor`]s and [`EmbedAuthorBuilder`]s.
pub trait EmbedAuthorExtension<T>: Sized {
    /// The type that will be returned if the embed author could not be created.
    type Error;

    /// Creates a new embed author from the given value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the author could not be created.
    fn parse(value: T) -> Result<Self, Self::Error>;
}

impl EmbedAuthorExtension<&CachedGuild> for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    fn parse(value: &CachedGuild) -> Result<Self, Self::Error> {
        Ok(Self::new(value.name()).icon_url(value.into_image_source()?))
    }
}

impl EmbedAuthorExtension<&CurrentUser> for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    fn parse(value: &CurrentUser) -> Result<Self, Self::Error> {
        Ok(Self::new(&value.name).icon_url(value.into_image_source()?))
    }
}

impl EmbedAuthorExtension<&Guild> for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    fn parse(value: &Guild) -> Result<Self, Self::Error> {
        Ok(Self::new(&value.name).icon_url(value.into_image_source()?))
    }
}

impl EmbedAuthorExtension<(&Member, Id<GuildMarker>)> for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    fn parse((member, guild_id): (&Member, Id<GuildMarker>)) -> Result<Self, Self::Error> {
        Ok(Self::new(member.nick.as_ref().unwrap_or(&member.user.name))
            .icon_url(member.into_image_source(guild_id)?))
    }
}

impl EmbedAuthorExtension<(&PartialMember, Id<GuildMarker>)> for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    fn parse((member, guild_id): (&PartialMember, Id<GuildMarker>)) -> Result<Self, Self::Error> {
        let Some(name) = member.nick.as_ref().or_else(|| member.user.as_ref().map(|u| &u.name))
        else {
            bail!("cannot resolve member name");
        };

        Ok(Self::new(name).icon_url(member.into_image_source(guild_id)?))
    }
}

impl EmbedAuthorExtension<&User> for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    fn parse(value: &User) -> Result<Self, Self::Error> {
        Ok(Self::new(&value.name).icon_url(value.into_image_source()?))
    }
}

impl<T> EmbedAuthorExtension<T> for EmbedAuthor
where
    EmbedAuthorBuilder: EmbedAuthorExtension<T>,
{
    type Error = <EmbedAuthorBuilder as EmbedAuthorExtension<T>>::Error;

    #[inline]
    fn parse(value: T) -> Result<Self, Self::Error> {
        Ok(EmbedAuthorBuilder::parse(value)?.build())
    }
}

/// Provides type extensions for [`Interaction`]s.
pub trait InteractionExtension {
    /// Provides a marker string for the interaction.
    fn marker(&self) -> String;
}

impl InteractionExtension for Interaction {
    fn marker(&self) -> String {
        self.author_id().map_or_else(
            || format!("<{:?} #{}>", self.kind, self.id),
            |id| format!("<{:?} #{} @{id}>", self.kind, self.id),
        )
    }
}

impl InteractionExtension for InteractionCreate {
    #[inline]
    fn marker(&self) -> String {
        self.0.marker()
    }
}

/// Provides type extensions for [`ReactionType`]s.
pub trait ReactionTypeExtension<T>: Sized {
    /// The type returned if a reaction type cannot be parsed.
    type Error;

    /// Returns the reaction type represented by the given value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be parsed.
    fn parse(value: T) -> Result<Self, Self::Error>;
}

impl ReactionTypeExtension<char> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: char) -> Result<Self, Self::Error> {
        Ok(Self::Unicode { name: value.to_string() })
    }
}

impl ReactionTypeExtension<&str> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: &str) -> Result<Self, Self::Error> {
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

impl ReactionTypeExtension<String> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: String) -> Result<Self, Self::Error> {
        Self::parse(&(*value))
    }
}

impl ReactionTypeExtension<Box<str>> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: Box<str>) -> Result<Self, Self::Error> {
        Self::parse(&(*value))
    }
}

impl ReactionTypeExtension<Cow<'_, str>> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: Cow<str>) -> Result<Self, Self::Error> {
        Self::parse(&(*value))
    }
}

impl ReactionTypeExtension<&String> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: &String) -> Result<Self, Self::Error> {
        Self::parse(&(**value))
    }
}

impl ReactionTypeExtension<&Box<str>> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: &Box<str>) -> Result<Self, Self::Error> {
        Self::parse(&(**value))
    }
}

impl ReactionTypeExtension<&Cow<'_, str>> for ReactionType {
    type Error = anyhow::Error;

    fn parse(value: &Cow<str>) -> Result<Self, Self::Error> {
        Self::parse(&(**value))
    }
}

/// Provides type extensions for [`str`]s.
pub trait StrExtension {
    /// Collapses common plaintext escape sequences into their characters.
    fn collapse(&self) -> String;
}

impl StrExtension for &str {
    fn collapse(&self) -> String {
        self.replace(r"\n", "\n").replace(r"\t", "\t").replace(r"\r", "\r")
    }
}

/// Provides type extensions for users.
pub trait UserExtension {
    /// Returns the displayed name of this user, defaulting to their tag.
    fn display(&self) -> String;

    /// Returns the tag of this [`User`], either in the current format (`@username`) or old format
    /// (`Username#1234`).
    fn tag(&self) -> String;

    /// Returns the accent color of this [`User`], or the bot's default brand color.
    fn color(&self) -> u32;
}

impl UserExtension for CurrentUser {
    #[inline]
    fn display(&self) -> String {
        self.tag()
    }

    fn tag(&self) -> String {
        if self.discriminator == 0 {
            format!("@{}", self.name)
        } else {
            format!("{}#{}", self.name, self.discriminator())
        }
    }

    #[inline]
    fn color(&self) -> u32 {
        self.accent_color.unwrap_or(BRANDING)
    }
}

impl UserExtension for Member {
    #[inline]
    fn display(&self) -> String {
        self.nick.as_deref().map_or_else(|| self.user.display(), Into::into)
    }

    #[inline]
    fn tag(&self) -> String {
        self.user.tag()
    }

    #[inline]
    fn color(&self) -> u32 {
        self.user.color()
    }
}

impl UserExtension for User {
    #[inline]
    fn display(&self) -> String {
        self.global_name.as_deref().map_or_else(|| self.tag(), Into::into)
    }

    fn tag(&self) -> String {
        if self.discriminator == 0 {
            format!("@{}", self.name)
        } else {
            format!("{}#{}", self.name, self.discriminator())
        }
    }

    #[inline]
    fn color(&self) -> u32 {
        self.accent_color.unwrap_or(BRANDING)
    }
}

impl<T: UserExtension> UserExtension for &T {
    #[inline]
    fn display(&self) -> String {
        <T as UserExtension>::display(self)
    }

    #[inline]
    fn tag(&self) -> String {
        <T as UserExtension>::tag(self)
    }

    #[inline]
    fn color(&self) -> u32 {
        <T as UserExtension>::color(self)
    }
}
