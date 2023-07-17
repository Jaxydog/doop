use std::convert::Infallible;

use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;
use twilight_model::user::{CurrentUser, DiscriminatorDisplay, PremiumType, User, UserFlags};
use twilight_model::util::ImageHash;
use twilight_util::builder::embed::EmbedAuthorBuilder;

use crate::extend::UserExt;
use crate::utility::Result;

/// Defines a value that can be created from a user.
pub trait TryFromUser: Sized {
    /// The error type that may result from the conversion.
    type Error;

    /// Creates a value from the provided user.
    fn try_from_user(user: &impl UserLike) -> Result<Self, Self::Error>;
}

/// Defines a value that can be created from a user.
pub trait FromUser {
    /// Creates a value from the provided user.
    fn from_user(user: &impl UserLike) -> Self;
}

impl<T: FromUser + Sized> TryFromUser for T {
    type Error = Infallible;

    #[inline]
    fn try_from_user(user: &impl UserLike) -> Result<Self, Self::Error> {
        Ok(Self::from_user(user))
    }
}

impl TryFromUser for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_user(user: &impl UserLike) -> Result<Self, Self::Error> {
        Ok(Self::new(if user.discriminator().get() == 0 {
            format!("@{}", user.name())
        } else {
            format!("{}#{}", user.name(), user.discriminator())
        })
        .icon_url(user.face()?))
    }
}

/// Marks a value as being user-like.
#[allow(missing_docs)]
pub trait UserLike {
    fn accent_color(&self) -> Option<&u32>;
    fn avatar(&self) -> Option<&ImageHash>;
    fn banner(&self) -> Option<&ImageHash>;
    fn bot(&self) -> &bool;
    fn discriminator(&self) -> DiscriminatorDisplay;
    fn id(&self) -> &Id<UserMarker>;
    fn email(&self) -> Option<&String>;
    fn flags(&self) -> Option<&UserFlags>;
    fn locale(&self) -> Option<&String>;
    fn mfa_enabled(&self) -> Option<&bool>;
    fn name(&self) -> &String;
    fn premium_type(&self) -> Option<&PremiumType>;
    fn public_flags(&self) -> Option<&UserFlags>;
    fn system(&self) -> Option<&bool>;
    fn verified(&self) -> Option<&bool>;
}

impl UserLike for User {
    #[inline]
    fn accent_color(&self) -> Option<&u32> {
        self.accent_color.as_ref()
    }

    #[inline]
    fn avatar(&self) -> Option<&ImageHash> {
        self.avatar.as_ref()
    }

    #[inline]
    fn banner(&self) -> Option<&ImageHash> {
        self.banner.as_ref()
    }

    #[inline]
    fn bot(&self) -> &bool {
        &self.bot
    }

    #[inline]
    fn discriminator(&self) -> DiscriminatorDisplay {
        self.discriminator()
    }

    #[inline]
    fn id(&self) -> &Id<UserMarker> {
        &self.id
    }

    #[inline]
    fn email(&self) -> Option<&String> {
        self.email.as_ref()
    }

    #[inline]
    fn flags(&self) -> Option<&UserFlags> {
        self.flags.as_ref()
    }

    #[inline]
    fn locale(&self) -> Option<&String> {
        self.locale.as_ref()
    }

    #[inline]
    fn mfa_enabled(&self) -> Option<&bool> {
        self.mfa_enabled.as_ref()
    }

    #[inline]
    fn name(&self) -> &String {
        &self.name
    }

    #[inline]
    fn premium_type(&self) -> Option<&PremiumType> {
        self.premium_type.as_ref()
    }

    #[inline]
    fn public_flags(&self) -> Option<&UserFlags> {
        self.public_flags.as_ref()
    }

    #[inline]
    fn system(&self) -> Option<&bool> {
        self.system.as_ref()
    }

    #[inline]
    fn verified(&self) -> Option<&bool> {
        self.verified.as_ref()
    }
}

impl UserLike for CurrentUser {
    #[inline]
    fn accent_color(&self) -> Option<&u32> {
        self.accent_color.as_ref()
    }

    #[inline]
    fn avatar(&self) -> Option<&ImageHash> {
        self.avatar.as_ref()
    }

    #[inline]
    fn banner(&self) -> Option<&ImageHash> {
        self.banner.as_ref()
    }

    #[inline]
    fn bot(&self) -> &bool {
        &self.bot
    }

    #[inline]
    fn discriminator(&self) -> DiscriminatorDisplay {
        self.discriminator()
    }

    #[inline]
    fn id(&self) -> &Id<UserMarker> {
        &self.id
    }

    #[inline]
    fn email(&self) -> Option<&String> {
        self.email.as_ref()
    }

    #[inline]
    fn flags(&self) -> Option<&UserFlags> {
        self.flags.as_ref()
    }

    #[inline]
    fn locale(&self) -> Option<&String> {
        self.locale.as_ref()
    }

    #[inline]
    fn mfa_enabled(&self) -> Option<&bool> {
        Some(&self.mfa_enabled)
    }

    #[inline]
    fn name(&self) -> &String {
        &self.name
    }

    #[inline]
    fn premium_type(&self) -> Option<&PremiumType> {
        self.premium_type.as_ref()
    }

    #[inline]
    fn public_flags(&self) -> Option<&UserFlags> {
        self.public_flags.as_ref()
    }

    #[inline]
    fn system(&self) -> Option<&bool> {
        None
    }

    #[inline]
    fn verified(&self) -> Option<&bool> {
        self.verified.as_ref()
    }
}
