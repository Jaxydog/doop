use twilight_model::guild::Guild;

use crate::utility::Result;

/// A simple, fallible conversion from the provided guild reference.
pub trait TryFromGuild<G: AsRef<Guild>>: Sized {
    /// The error type that may result from the conversion.
    type Error;

    /// Converts the provided referenced guild into the value.
    fn try_from_guild(guild: G) -> Result<Self, Self::Error>;
}

/// A simple conversion from the provided guild reference.
pub trait FromGuild<G: AsRef<Guild>>: Sized {
    /// Converts the provided referenced guild into the value.
    fn from_guild(guild: G) -> Self;
}

impl<G: AsRef<Guild>, T: TryFrom<G>> TryFromGuild<G> for T {
    type Error = <T as TryFrom<G>>::Error;

    #[inline]
    fn try_from_guild(guild: G) -> Result<Self, Self::Error> {
        Self::try_from(guild)
    }
}

impl<G: AsRef<Guild>, T: From<G>> FromGuild<G> for T {
    #[inline]
    fn from_guild(guild: G) -> Self {
        Self::from(guild)
    }
}
