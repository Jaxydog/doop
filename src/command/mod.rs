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

/// The application command.
pub mod application;
/// The embed command.
pub mod embed;
/// The help command.
pub mod help;
/// The ping command.
pub mod ping;
/// The role command.
pub mod role;

/// Implements a bot command.
pub trait CommandState {
    /// The command's name.
    fn name(&self) -> &'static str;
    /// The command's kind.
    fn kind(&self) -> CommandType;
    /// Builds and returns a constructed command value.
    fn build(&self, guild_id: Option<Id<GuildMarker>>) -> Result<Command>;
}

/// Utility wrapper structure for resolving command options
#[derive(Clone, Debug)]
pub struct CommandOptionResolver<'cmd> {
    data: &'cmd CommandData,
    mapped: HashMap<&'cmd str, &'cmd CommandOptionValue>,
}

impl<'cmd> CommandOptionResolver<'cmd> {
    /// Creates a new command option resolver
    #[inline]
    #[must_use]
    pub fn new(data: &'cmd CommandData) -> Self {
        Self::new_from(data, &data.options)
    }

    /// Creates a new command option resolver with the given options
    #[must_use]
    fn new_from(data: &'cmd CommandData, options: &'cmd [CommandDataOption]) -> Self {
        let resolved = options.iter().map(|option| {
            let CommandDataOption { name, value } = option;

            (name.as_str(), value)
        });

        Self { data, mapped: resolved.collect() }
    }

    /// Returns a resolved value from the command options
    #[inline]
    fn get(&'cmd self, name: &str) -> Result<&'cmd CommandOptionValue> {
        let Some(value) = self.mapped.get(name) else {
            return Err(anyhow!("missing value for option '{name}'"));
        };

        Ok(*value)
    }

    /// Returns a resolved subcommand from the command options
    #[inline]
    pub fn get_subcommand(&'cmd self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommand(v) = self.get(name)? else {
            return Err(anyhow!("invalid type for option '{name}'"));
        };

        Ok(Self::new_from(self.data, v))
    }

    /// Returns a resolved subcommand group from the command options
    #[inline]
    pub fn get_subgroup(&'cmd self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommandGroup(v) = self.get(name)? else {
            return Err(anyhow!("invalid type for option '{name}'"));
        };

        Ok(Self::new_from(self.data, v))
    }
}

/// Returns the focused field of the given command data.
#[inline]
#[must_use]
pub fn find_focused(data: &CommandData) -> Option<(&String, CommandOptionType)> {
    data.options.iter().find_map(|c| {
        let CommandOptionValue::Focused(ref name, kind) = c.value else {
            return None;
        };

        Some((name, kind))
    })
}

/// Generates a getter function for a command option resolver
macro_rules! resolve_getter {
    {$(
        $(#[$attribute:meta])*
        fn $name:ident() -> $variant:ident as $type:ty;
    )*} => {$(
        $(#[$attribute])*
        #[inline]
        pub fn $name(&'cmd self, name: &str) -> Result<&'cmd $type> {
            let CommandOptionValue::$variant(v) = self.get(name)? else {
                return Err(anyhow!("invalid type for option '{name}'"));
            };

            Ok(v)
        }
    )*};
}

impl<'cmd> CommandOptionResolver<'cmd> {
    resolve_getter! {
        /// Returns a resolved attachment identifier from the command options
        fn get_attachment_id() -> Attachment as Id<AttachmentMarker>;
        /// Returns a resolved bool from the command options
        fn get_bool() -> Boolean as bool;
        /// Returns a resolved channel identifier from the command options
        fn get_channel_id() -> Channel as Id<ChannelMarker>;
        /// Returns a resolved i64 from the command options
        fn get_i64() -> Integer as i64;
        /// Returns a resolved mentionable identifier from the command options
        fn get_mentionable_id() -> Mentionable as Id<GenericMarker>;
        /// Returns a resolved f64 from the command options
        fn get_f64() -> Number as f64;
        /// Returns a resolved role identifier from the command options
        fn get_role_id() -> Role as Id<RoleMarker>;
        /// Returns a resolved string from the command options
        fn get_string() -> String as String;
        /// Returns a resolved user identifier from the command options
        fn get_user_id() -> User as Id<UserMarker>;
    }
}
