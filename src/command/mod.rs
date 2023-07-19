use std::collections::HashMap;

use anyhow::anyhow;
use twilight_model::application::command::{Command, CommandOptionType, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, GenericMarker, GuildMarker, RoleMarker, UserMarker,
};
use twilight_model::id::Id;

use crate::utility::Result;

/// The embed command.
pub mod embed;
/// The help command.
pub mod help;
/// The ping command.
pub mod ping;
/// The role command.
pub mod role;

/// Defines a bot command.
pub trait DoopCommand: Send + Sync {
    /// The command's internal name.
    fn name(&self) -> &'static str;
    /// The command's API command type.
    fn kind(&self) -> CommandType;
    /// Builds and returns API command data.
    fn build(&self, guild_id: Option<Id<GuildMarker>>) -> Result<Command>;
}

/// A structure for resolving a command options.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandOptionResolver<'cmd> {
    /// The inner command data.
    data: &'cmd CommandData,
    /// The inner option cache.
    options: HashMap<&'cmd str, &'cmd CommandOptionValue>,
}

impl<'cmd> CommandOptionResolver<'cmd> {
    /// Creates a new [`CommandOptionResolver`].
    #[inline]
    #[must_use]
    pub fn new(data: &'cmd CommandData) -> Self { Self::new_from(data, &data.options) }

    /// Creates a new [`CommandOptionResolver`] with the given options.
    #[must_use]
    pub fn new_from(data: &'cmd CommandData, options: &'cmd [CommandDataOption]) -> Self {
        let options = options.iter().map(|o| (o.name.as_str(), &o.value));

        Self { data, options: options.collect() }
    }

    /// Returns a resolved value from the command's options.
    #[inline]
    fn get(&'cmd self, name: &str) -> Result<&'cmd CommandOptionValue> {
        let Some(value) = self.options.get(name) else {
            return Err(anyhow!("missing value for option '{name}'"));
        };

        Ok(*value)
    }

    /// Returns a new [`CommandOptionResolver`] containing the requested
    /// subcommand's options.
    #[inline]
    pub fn get_subcommand(&'cmd self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommand(ref v) = self.get(name)? else {
            return Err(anyhow!("invalid type for option '{name}'"));
        };

        Ok(Self::new_from(self.data, v))
    }

    /// Returns a new [`CommandOptionResolver`] containing the requested
    /// subcommand group's options.
    #[inline]
    pub fn get_subcommand_group(&'cmd self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommandGroup(ref v) = self.get(name)? else {
            return Err(anyhow!("invalid type for option '{name}'"));
        };

        Ok(Self::new_from(self.data, v))
    }
}

/// Generates getter methods for the [`CommandOptionResolver`] type.
///
/// ```
/// getter! {
///     /// Getter method.
///     fn get_bool() -> Boolean as bool;
/// }
/// ```
macro_rules! getter {
    {$(
        $(#[$attribute:meta])*
        fn $fn:ident() -> $variant:ident as $return:ty;
    )*} => {
        impl<'cmd> CommandOptionResolver<'cmd> {$(
            $(#[$attribute])*
            #[inline]
            pub fn $fn(&'cmd self, name: &str) -> Result<&'cmd $return> {
                let CommandOptionValue::$variant(ref v) = self.get(name)? else {
                    return Err(anyhow!("invalid type for option '{name}'"));
                };

                Ok(v)
            }
        )*}
    };
}

getter! {
    /// Returns a reference to the requested attachment identifier.
    fn get_attachment_id() -> Attachment as Id<AttachmentMarker>;
    /// Returns a reference to the requested boolean.
    fn get_bool() -> Boolean as bool;
    /// Returns a reference to the requested channel identifier.
    fn get_channel_id() -> Channel as Id<ChannelMarker>;
    /// Returns a reference to the requested integer.
    fn get_i64() -> Integer as i64;
    /// Returns a reference to the requested mentionable identifier.
    fn get_mentionable_id() -> Mentionable as Id<GenericMarker>;
    /// Returns a reference to the requested number.
    fn get_f64() -> Number as f64;
    /// Returns a reference to the requested role identifier.
    fn get_role_id() -> Role as Id<RoleMarker>;
    /// Returns a reference to the requested string.
    fn get_str() -> String as str;
    /// Returns a reference to the requested user identifier.
    fn get_user_id() -> User as Id<UserMarker>;
}

/// Returns the focused field of the given command data.
#[inline]
pub fn find_focused(data: &CommandData) -> Result<(&str, CommandOptionType)> {
    data.options
        .iter()
        .find_map(|c| {
            let CommandOptionValue::Focused(ref name, kind) = c.value else {
                return None;
            };

            Some((name.as_str(), kind))
        })
        .ok_or_else(|| anyhow!("missing focused command option"))
}
