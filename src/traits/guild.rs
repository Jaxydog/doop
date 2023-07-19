use std::convert::Infallible;

use twilight_cache_inmemory::model::CachedGuild;
use twilight_model::guild::{
    AfkTimeout, DefaultMessageNotificationLevel, ExplicitContentFilter, Guild, MfaLevel, NSFWLevel,
    Permissions, PremiumTier, SystemChannelFlags, VerificationLevel,
};
use twilight_model::id::marker::{ApplicationMarker, ChannelMarker, GuildMarker, UserMarker};
use twilight_model::id::Id;
use twilight_model::util::{ImageHash, Timestamp};
use twilight_util::builder::embed::EmbedAuthorBuilder;

use crate::extend::GuildExt;
use crate::utility::Result;

/// A simple, fallible conversion from the provided guild reference.
pub trait TryFromGuild: Sized {
    /// The error type that may result from the conversion.
    type Error;

    /// Converts the provided referenced guild into the value.
    fn try_from_guild(guild: &impl GuildLike) -> Result<Self, Self::Error>;
}

/// A simple conversion from the provided guild reference.
pub trait FromGuild: Sized {
    /// Converts the provided referenced guild into the value.
    fn from_guild(guild: &impl GuildLike) -> Self;
}

impl<T: FromGuild> TryFromGuild for T {
    type Error = Infallible;

    #[inline]
    fn try_from_guild(guild: &impl GuildLike) -> Result<Self, Self::Error> {
        Ok(Self::from_guild(guild))
    }
}

impl TryFromGuild for EmbedAuthorBuilder {
    type Error = anyhow::Error;

    #[inline]
    fn try_from_guild(guild: &impl GuildLike) -> Result<Self, Self::Error> {
        Ok(Self::new(guild.name()).icon_url(guild.image()?))
    }
}

/// Marks a value as being guild-like.
#[allow(missing_docs)]
pub trait GuildLike {
    fn afk_channel_id(&self) -> Option<Id<ChannelMarker>>;
    fn afk_timeout(&self) -> AfkTimeout;
    fn application_id(&self) -> Option<Id<ApplicationMarker>>;
    fn banner(&self) -> Option<&ImageHash>;
    fn default_message_notifications(&self) -> DefaultMessageNotificationLevel;
    fn description(&self) -> Option<&str>;
    fn discovery_splash(&self) -> Option<&ImageHash>;
    fn explicit_content_filter(&self) -> ExplicitContentFilter;
    fn icon(&self) -> Option<&ImageHash>;
    fn id(&self) -> Id<GuildMarker>;
    fn joined_at(&self) -> Option<Timestamp>;
    fn large(&self) -> bool;
    fn max_members(&self) -> Option<u64>;
    fn max_presences(&self) -> Option<u64>;
    fn max_video_channel_users(&self) -> Option<u64>;
    fn member_count(&self) -> Option<u64>;
    fn mfa_level(&self) -> MfaLevel;
    fn name(&self) -> &str;
    fn nsfw_level(&self) -> NSFWLevel;
    fn owner_id(&self) -> Id<UserMarker>;
    fn owner(&self) -> Option<bool>;
    fn permissions(&self) -> Option<Permissions>;
    fn preferred_locale(&self) -> &str;
    fn premium_progress_bar_enabled(&self) -> bool;
    fn premium_subscription_count(&self) -> Option<u64>;
    fn premium_tier(&self) -> PremiumTier;
    fn public_updates_channel_id(&self) -> Option<Id<ChannelMarker>>;
    fn rules_channel_id(&self) -> Option<Id<ChannelMarker>>;
    fn splash(&self) -> Option<&ImageHash>;
    fn system_channel_flags(&self) -> SystemChannelFlags;
    fn system_channel_id(&self) -> Option<Id<ChannelMarker>>;
    fn unavailable(&self) -> bool;
    fn vanity_url_code(&self) -> Option<&str>;
    fn verification_level(&self) -> VerificationLevel;
    fn widget_channel_id(&self) -> Option<Id<ChannelMarker>>;
    fn widget_enabled(&self) -> Option<bool>;
}

impl GuildLike for Guild {
    #[inline]
    fn afk_channel_id(&self) -> Option<Id<ChannelMarker>> { self.afk_channel_id }

    #[inline]
    fn afk_timeout(&self) -> AfkTimeout { self.afk_timeout }

    #[inline]
    fn application_id(&self) -> Option<Id<ApplicationMarker>> { self.application_id }

    #[inline]
    fn banner(&self) -> Option<&ImageHash> { self.banner.as_ref() }

    #[inline]
    fn default_message_notifications(&self) -> DefaultMessageNotificationLevel {
        self.default_message_notifications
    }

    #[inline]
    fn description(&self) -> Option<&str> { self.description.as_deref() }

    #[inline]
    fn discovery_splash(&self) -> Option<&ImageHash> { self.discovery_splash.as_ref() }

    #[inline]
    fn explicit_content_filter(&self) -> ExplicitContentFilter { self.explicit_content_filter }

    #[inline]
    fn icon(&self) -> Option<&ImageHash> { self.icon.as_ref() }

    #[inline]
    fn id(&self) -> Id<GuildMarker> { self.id }

    #[inline]
    fn joined_at(&self) -> Option<Timestamp> { self.joined_at }

    #[inline]
    fn large(&self) -> bool { self.large }

    #[inline]
    fn max_members(&self) -> Option<u64> { self.max_members }

    #[inline]
    fn max_presences(&self) -> Option<u64> { self.max_presences }

    #[inline]
    fn max_video_channel_users(&self) -> Option<u64> { self.max_video_channel_users }

    #[inline]
    fn member_count(&self) -> Option<u64> { self.member_count }

    #[inline]
    fn mfa_level(&self) -> MfaLevel { self.mfa_level }

    #[inline]
    fn name(&self) -> &str { &self.name }

    #[inline]
    fn nsfw_level(&self) -> NSFWLevel { self.nsfw_level }

    #[inline]
    fn owner_id(&self) -> Id<UserMarker> { self.owner_id }

    #[inline]
    fn owner(&self) -> Option<bool> { self.owner }

    #[inline]
    fn permissions(&self) -> Option<Permissions> { self.permissions }

    #[inline]
    fn preferred_locale(&self) -> &str { &self.preferred_locale }

    #[inline]
    fn premium_progress_bar_enabled(&self) -> bool { self.premium_progress_bar_enabled }

    #[inline]
    fn premium_subscription_count(&self) -> Option<u64> { self.premium_subscription_count }

    #[inline]
    fn premium_tier(&self) -> PremiumTier { self.premium_tier }

    #[inline]
    fn public_updates_channel_id(&self) -> Option<Id<ChannelMarker>> {
        self.public_updates_channel_id
    }

    #[inline]
    fn rules_channel_id(&self) -> Option<Id<ChannelMarker>> { self.rules_channel_id }

    #[inline]
    fn splash(&self) -> Option<&ImageHash> { self.splash.as_ref() }

    #[inline]
    fn system_channel_flags(&self) -> SystemChannelFlags { self.system_channel_flags }

    #[inline]
    fn system_channel_id(&self) -> Option<Id<ChannelMarker>> { self.system_channel_id }

    #[inline]
    fn unavailable(&self) -> bool { self.unavailable }

    #[inline]
    fn vanity_url_code(&self) -> Option<&str> { self.vanity_url_code.as_deref() }

    #[inline]
    fn verification_level(&self) -> VerificationLevel { self.verification_level }

    #[inline]
    fn widget_channel_id(&self) -> Option<Id<ChannelMarker>> { self.widget_channel_id }

    #[inline]
    fn widget_enabled(&self) -> Option<bool> { self.widget_enabled }
}

impl GuildLike for CachedGuild {
    #[inline]
    fn afk_channel_id(&self) -> Option<Id<ChannelMarker>> { self.afk_channel_id() }

    #[inline]
    fn afk_timeout(&self) -> AfkTimeout { self.afk_timeout() }

    #[inline]
    fn application_id(&self) -> Option<Id<ApplicationMarker>> { self.application_id() }

    #[inline]
    fn banner(&self) -> Option<&ImageHash> { self.banner() }

    #[inline]
    fn default_message_notifications(&self) -> DefaultMessageNotificationLevel {
        self.default_message_notifications()
    }

    #[inline]
    fn description(&self) -> Option<&str> { self.description() }

    #[inline]
    fn discovery_splash(&self) -> Option<&ImageHash> { self.discovery_splash() }

    #[inline]
    fn explicit_content_filter(&self) -> ExplicitContentFilter { self.explicit_content_filter() }

    #[inline]
    fn icon(&self) -> Option<&ImageHash> { self.icon() }

    #[inline]
    fn id(&self) -> Id<GuildMarker> { self.id() }

    #[inline]
    fn joined_at(&self) -> Option<Timestamp> { self.joined_at() }

    #[inline]
    fn large(&self) -> bool { self.large() }

    #[inline]
    fn max_members(&self) -> Option<u64> { self.max_members() }

    #[inline]
    fn max_presences(&self) -> Option<u64> { self.max_presences() }

    #[inline]
    fn max_video_channel_users(&self) -> Option<u64> { self.max_video_channel_users() }

    #[inline]
    fn member_count(&self) -> Option<u64> { self.member_count() }

    #[inline]
    fn mfa_level(&self) -> MfaLevel { self.mfa_level() }

    #[inline]
    fn name(&self) -> &str { self.name() }

    #[inline]
    fn nsfw_level(&self) -> NSFWLevel { self.nsfw_level() }

    #[inline]
    fn owner_id(&self) -> Id<UserMarker> { self.owner_id() }

    #[inline]
    fn owner(&self) -> Option<bool> { self.owner() }

    #[inline]
    fn permissions(&self) -> Option<Permissions> { self.permissions() }

    #[inline]
    fn preferred_locale(&self) -> &str { self.preferred_locale() }

    #[inline]
    fn premium_progress_bar_enabled(&self) -> bool { self.premium_progress_bar_enabled() }

    #[inline]
    fn premium_subscription_count(&self) -> Option<u64> { self.premium_subscription_count() }

    #[inline]
    fn premium_tier(&self) -> PremiumTier { self.premium_tier() }

    #[inline]
    fn public_updates_channel_id(&self) -> Option<Id<ChannelMarker>> {
        self.public_updates_channel_id()
    }

    #[inline]
    fn rules_channel_id(&self) -> Option<Id<ChannelMarker>> { self.rules_channel_id() }

    #[inline]
    fn splash(&self) -> Option<&ImageHash> { self.splash() }

    #[inline]
    fn system_channel_flags(&self) -> SystemChannelFlags { self.system_channel_flags() }

    #[inline]
    fn system_channel_id(&self) -> Option<Id<ChannelMarker>> { self.system_channel_id() }

    #[inline]
    fn unavailable(&self) -> bool { self.unavailable() }

    #[inline]
    fn vanity_url_code(&self) -> Option<&str> { self.vanity_url_code() }

    #[inline]
    fn verification_level(&self) -> VerificationLevel { self.verification_level() }

    #[inline]
    fn widget_channel_id(&self) -> Option<Id<ChannelMarker>> { self.widget_channel_id() }

    #[inline]
    fn widget_enabled(&self) -> Option<bool> { self.widget_enabled() }
}
