use std::convert::Infallible;

use twilight_model::guild::Guild;
use twilight_util::builder::embed::EmbedAuthorBuilder;

use crate::extend::GuildExt;
use crate::utility::Result;

/// A simple, fallible conversion from the provided guild reference.
pub trait TryFromGuild: Sized {
    /// The error type that may result from the conversion.
    type Error;

    /// Converts the provided referenced guild into the value.
    fn try_from_guild(guild: &Guild) -> Result<Self, Self::Error>;
}

/// A simple conversion from the provided guild reference.
pub trait FromGuild: Sized {
    /// Converts the provided referenced guild into the value.
    fn from_guild(guild: &Guild) -> Self;
}

impl<T: FromGuild> TryFromGuild for T {
    type Error = Infallible;

    #[inline]
    fn try_from_guild(guild: &Guild) -> Result<Self, Self::Error> {
        Ok(Self::from_guild(guild))
    }
}

impl TryFromGuild for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_guild(guild: &Guild) -> Result<Self, Self::Error> {
        Ok(Self::new(&guild.name).icon_url(guild.icon()?))
    }
}
