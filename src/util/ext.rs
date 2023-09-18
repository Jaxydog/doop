use std::borrow::Cow;

use anyhow::{anyhow, bail};
use twilight_cache_inmemory::model::CachedGuild;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::ReactionType;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::guild::{Guild, Member, PartialGuild};
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, User};
use twilight_util::builder::embed::EmbedAuthorBuilder;

use crate::util::traits::GetIcon;
use crate::util::{Result, BRANDING};

/// Provides extensions for [`EmbedAuthor`]s and [`EmbedAuthorBuilder`]s.
pub trait EmbedAuthorExt<T>: Sized {
    /// The type that will be returned if the embed author could not be created.
    type Error;

    /// Creates a new [`EmbedAuthor`] from the given value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the author could not be created.
    fn new_from(value: &T) -> Result<Self, Self::Error>;
}

impl EmbedAuthorExt<CachedGuild> for EmbedAuthorBuilder {
    type Error = <CachedGuild as GetIcon>::Error;

    #[inline]
    fn new_from(value: &CachedGuild) -> Result<Self, Self::Error> {
        Ok(Self::new(value.name()).icon_url(<CachedGuild as GetIcon>::get_icon(value)?))
    }
}

impl EmbedAuthorExt<Guild> for EmbedAuthorBuilder {
    type Error = <Guild as GetIcon>::Error;

    #[inline]
    fn new_from(value: &Guild) -> Result<Self, Self::Error> {
        Ok(Self::new(&value.name).icon_url(value.get_icon()?))
    }
}

impl EmbedAuthorExt<PartialGuild> for EmbedAuthorBuilder {
    type Error = <Guild as GetIcon>::Error;

    #[inline]
    fn new_from(value: &PartialGuild) -> Result<Self, Self::Error> {
        Ok(Self::new(&value.name).icon_url(value.get_icon()?))
    }
}

impl<T: GetIcon + UserExt> EmbedAuthorExt<T> for EmbedAuthorBuilder {
    type Error = <T as GetIcon>::Error;

    #[inline]
    fn new_from(value: &T) -> Result<Self, Self::Error> {
        Ok(Self::new(value.tag()).icon_url(value.get_icon()?))
    }
}

impl<T> EmbedAuthorExt<T> for EmbedAuthor
where
    EmbedAuthorBuilder: EmbedAuthorExt<T>,
{
    type Error = <EmbedAuthorBuilder as EmbedAuthorExt<T>>::Error;

    #[inline]
    fn new_from(value: &T) -> Result<Self, Self::Error> {
        Ok(EmbedAuthorBuilder::new_from(value)?.build())
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

/// Provides extensions for [`ReactionType`]s.
pub trait ReactionTypeExt<T>: Sized {
    /// Creates a new [`ReactionType`] from the given [`str`] reference.
    ///
    /// # Errors
    ///
    /// This function will return an error if the value could not be parsed.
    fn parse(value: T) -> Result<Self>;
}

impl ReactionTypeExt<String> for ReactionType {
    #[inline]
    fn parse(value: String) -> Result<Self> {
        Self::parse(&(*value))
    }
}

impl ReactionTypeExt<&String> for ReactionType {
    #[inline]
    fn parse(value: &String) -> Result<Self> {
        Self::parse(&(**value))
    }
}

impl ReactionTypeExt<Box<str>> for ReactionType {
    #[inline]
    fn parse(value: Box<str>) -> Result<Self> {
        Self::parse(&(*value))
    }
}

impl ReactionTypeExt<&Box<str>> for ReactionType {
    #[inline]
    fn parse(value: &Box<str>) -> Result<Self> {
        Self::parse(&(**value))
    }
}

impl ReactionTypeExt<Cow<'_, str>> for ReactionType {
    #[inline]
    fn parse(value: Cow<'_, str>) -> Result<Self> {
        Self::parse(&(*value))
    }
}

impl ReactionTypeExt<&str> for ReactionType {
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

impl ReactionTypeExt<char> for ReactionType {
    #[inline]
    fn parse(value: char) -> Result<Self> {
        Ok(Self::Unicode { name: value.to_string() })
    }
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
    /// Returns the displayed name of this user, defaulting to their tag.
    fn display(&self) -> String;

    /// Returns the tag of this [`User`], either in the current format (`@username`) or old format
    /// (`Username#1234`).
    fn tag(&self) -> String;

    // /// Returns the display name of this [`User`].
    // fn name(&self) -> String;

    /// Returns the accent color of this [`User`], or the bot's default brand color.
    fn color(&self) -> u32;
}

impl UserExt for CurrentUser {
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

impl UserExt for Member {
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

impl UserExt for User {
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

impl<T: UserExt> UserExt for &T {
    #[inline]
    fn display(&self) -> String {
        <T as UserExt>::display(self)
    }

    #[inline]
    fn tag(&self) -> String {
        <T as UserExt>::tag(self)
    }

    #[inline]
    fn color(&self) -> u32 {
        <T as UserExt>::color(self)
    }
}
