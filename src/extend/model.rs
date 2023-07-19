use anyhow::anyhow;
use time::ext::NumericalDuration;
use time::{OffsetDateTime, UtcOffset};
use twilight_model::channel::message::ReactionType;
use twilight_model::gateway::payload::incoming::InteractionCreate;
use twilight_model::id::Id;
use twilight_util::builder::embed::ImageSource;

use crate::traits::{GuildLike, UserLike};
use crate::utility::{Result, BRANDING_COLOR, CDN_BASE, DISCORD_EPOCH};

/// Provides additional methods for guilds.
pub trait GuildExt {
    /// Returns the guild's icon image source.
    fn image(&self) -> Result<ImageSource>;
}

impl<G: GuildLike> GuildExt for G {
    fn image(&self) -> Result<ImageSource> {
        let Some(hash) = self.icon() else {
            return Err(anyhow!("missing icon hash"));
        };

        let url = format!("{CDN_BASE}/icons/{}/{hash}.png", self.id());

        Ok(ImageSource::url(url)?)
    }
}

/// Provides additional methods for identifiers.
pub trait IdExt {
    /// Returns the identifier's creation date.
    fn created_at(&self) -> OffsetDateTime;
    /// Returns the identifier's creation date in the given UTC offset.
    fn created_at_in(&self, offset: impl Into<UtcOffset>) -> OffsetDateTime {
        self.created_at().to_offset(offset.into())
    }
}

impl<T> IdExt for Id<T> {
    #[inline]
    fn created_at(&self) -> OffsetDateTime {
        #[allow(clippy::cast_possible_wrap)]
        let ms = (self.get() >> 22) as i64;

        DISCORD_EPOCH.saturating_add(ms.milliseconds())
    }
}

/// Provides additional methods for interaction create events.
pub trait InteractionCreateExt {
    /// Returns the interaction create event's internal label.
    fn label(&self) -> String;
}

impl InteractionCreateExt for InteractionCreate {
    #[inline]
    fn label(&self) -> String { format!("<{:?}::{}>", self.kind, self.id) }
}

/// Provides additional methods for reaction types.
pub trait ReactionTypeExt<T>: Sized {
    /// Attempts to parse the given value into a reaction type.
    fn parse(value: T) -> Result<Self>;
}

impl ReactionTypeExt<&str> for ReactionType {
    fn parse(value: &str) -> Result<Self> {
        let mut chars = value.chars();
        let string = match chars.clone().count() {
            0 => return Err(anyhow!("expected a non-empty string")),
            #[allow(clippy::unwrap_used)] // there's always a next char
            1 => return Self::parse(chars.next().unwrap()),
            _ => chars.collect::<String>(),
        };

        if !string.starts_with('<') {
            return Ok(Self::Unicode { name: string });
        }
        if !string.ends_with('>') {
            return Err(anyhow!("invalid emoji formatting"));
        }

        let inner = string.trim_matches(&['<', '>'] as &[char]);
        let mut split = inner.split(':');

        let animated = if let Some(slice) = split.next() {
            slice == "a"
        } else {
            return Err(anyhow!("invalid emoji formatting"));
        };
        let Some(name) = split.next().map(ToString::to_string) else {
            return Err(anyhow!("invalid emoji formatting"));
        };
        let id = if let Some(id) = split.next() {
            Id::new_checked(id.parse()?).ok_or_else(|| anyhow!("expected non-zero identifier"))?
        } else {
            return Err(anyhow!("invalid emoji formatting"));
        };

        Ok(Self::Custom { animated, id, name: Some(name) })
    }
}

impl ReactionTypeExt<String> for ReactionType {
    fn parse(value: String) -> Result<Self> { Self::parse(value.as_str()) }
}

impl ReactionTypeExt<char> for ReactionType {
    fn parse(value: char) -> Result<Self> { Ok(Self::Unicode { name: value.to_string() }) }
}

/// Provides additional methods for users
pub trait UserExt {
    /// Returns the user's accent color, defaulting to the bot branding color
    fn color(&self) -> u32;
    /// Returns the user's avatar image source, defaulting to a standard avatar
    fn face(&self) -> Result<ImageSource>;
}

impl<U: UserLike> UserExt for U {
    #[inline]
    fn color(&self) -> u32 {
        self.accent_color()
            .copied()
            .unwrap_or_else(|| BRANDING_COLOR.into())
    }

    fn face(&self) -> Result<ImageSource> {
        let link = self.avatar().map_or_else(
            || format!("{CDN_BASE}/embed/avatars/{}.png", self.id().get() % 5),
            |hash| format!("{CDN_BASE}/avatars/{}/{hash}.png", self.id()),
        );

        Ok(ImageSource::url(link)?)
    }
}
