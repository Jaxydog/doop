use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::bail;
use doop_storage::{Compress, FileKey, MsgPack};
use serde::{Deserialize, Serialize};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::application::command::{Command, CommandOptionChoice, CommandOptionType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::application::interaction::Interaction;
use twilight_model::id::marker::{AttachmentMarker, ChannelMarker, GenericMarker, GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;
use uuid::Uuid;

use crate::util::Result;

/// A basic event handler.
pub trait Handler: Send + Sync {
    /// The name of this [`Handler`].
    fn name(&self) -> &'static str;
}

/// A value that can be converted into a Discord [`Command`].
pub trait AsCommand {
    /// Builds and returns the [`Command`] representation of this value.
    ///
    /// # Errors
    ///
    /// This function will return an error if the command could not be created.
    fn command(&self, guild_id: Option<Id<GuildMarker>>) -> Result<Option<Command>>;
}

/// A basic interaction handler.
#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait InteractionHandler: AsCommand + Handler {
    /// Handles an autocompletion interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be handled.
    async fn handle_autocomplete<'api: 'evt, 'evt>(
        &self,
        ctx: CommandCtx<'api, 'evt>,
        focus: (&'evt str, CommandOptionType),
    ) -> Result<Vec<CommandOptionChoice>> {
        bail!("unhandleable inteaction type")
    }

    /// Handles a command interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be handled.
    async fn handle_command<'api: 'evt, 'evt>(&self, ctx: CommandCtx<'api, 'evt>) -> Result {
        bail!("unhandleable inteaction type")
    }

    /// Handles a component interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be handled.
    async fn handle_component<'api: 'evt, 'evt>(&self, ctx: ComponentCtx<'api, 'evt>, cid: CId) -> Result {
        bail!("unhandleable inteaction type")
    }

    /// Handles a modal interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be handled.
    async fn handle_modal<'api: 'evt, 'evt>(&self, ctx: ModalCtx<'api, 'evt>, cid: CId) -> Result {
        bail!("unhandleable inteaction type")
    }
}

/// A reference to the bot's HTTP API and cache instance.
#[derive(Clone, Copy, Debug)]
pub struct Api<'api> {
    /// The API's HTTP value.
    http: &'api Arc<Client>,
    /// The API's cache value.
    cache: &'api Arc<InMemoryCache>,
}

impl<'api> Api<'api> {
    /// Creates a new [`Api`].
    #[inline]
    pub const fn new(http: &'api Arc<Client>, cache: &'api Arc<InMemoryCache>) -> Self {
        Self { http, cache }
    }

    /// Returns a reference to the HTTP client of this [`Api`].
    #[inline]
    #[must_use]
    pub const fn http(&self) -> &'api Arc<Client> {
        self.http
    }

    /// Returns a reference to the cache of this [`Api`].
    #[inline]
    #[must_use]
    pub const fn cache(&self) -> &'api Arc<InMemoryCache> {
        self.cache
    }
}

/// A command interaction event context.
pub type CommandCtx<'api, 'evt> = Ctx<'api, 'evt, &'evt CommandData>;

/// A component interaction event context.
pub type ComponentCtx<'api, 'evt> = Ctx<'api, 'evt, &'evt MessageComponentInteractionData>;

/// A modal interaction event context.
pub type ModalCtx<'api, 'evt> = Ctx<'api, 'evt, &'evt ModalInteractionData>;

/// An event context.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'api: 'evt, 'evt, T> {
    /// The HTTP API of this [`Ctx<T>`].
    pub api: Api<'api>,
    /// The referenced event of this [`Ctx<T>`].
    pub event: &'evt Interaction,
    /// The data of this [`Ctx<T>`].
    pub data: T,
}

impl<'api: 'evt, 'evt, T> Ctx<'api, 'evt, T> {
    /// Creates a new [`Ctx<T>`].
    #[inline]
    pub const fn new(api: Api<'api>, event: &'evt Interaction, data: T) -> Self {
        Self { api, event, data }
    }

    /// Returns the interaction client of this [`Ctx<T>`].
    #[inline]
    pub fn client(&self) -> InteractionClient {
        self.api.http().interaction(self.event.application_id)
    }
}

/// A custom identifier with data storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CId {
    /// The name of the handler and its component.
    name: (Box<str>, Box<str>),
    /// The internal stringified data.
    data: Vec<Box<str>>,
    /// The internal storage key identifier.
    uuid: Option<Uuid>,
}

impl CId {
    /// The maximum length of an identifier in bytes.
    pub const MAX_LEN: usize = 100;
    /// The character used to separate each part of the identifier.
    pub const PART_SEP: char = '$';
    /// The character used to serparate data values within the identifier.
    pub const DATA_SEP: char = ';';

    /// Creates a new [`CId`].
    pub fn new(handler: impl AsRef<str>, component: impl AsRef<str>) -> Self {
        let name = (handler.as_ref().into(), component.as_ref().into());

        Self { name, data: vec![], uuid: None }
    }

    /// Returns a reference to the event handler name of this [`CId`].
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &str {
        &self.name.0
    }

    /// Returns a reference to the component kind of this [`CId`].
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> &str {
        &self.name.1
    }

    /// Returns the data at the given index.
    #[inline]
    #[must_use]
    pub fn data(&self, index: usize) -> Option<&str> {
        self.data.get(index).map(|b| &(**b))
    }

    /// Returns the storage key of this [`CId`].
    #[inline]
    #[must_use]
    pub fn key<T>(&self) -> Option<FileKey<T, Compress<MsgPack, 4>>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        Some(format!(".cid/{}/{}/{}", self.name.0, self.name.1, self.uuid?).into())
    }

    /// Generates a new random storage key for this [`CId`].
    #[must_use]
    pub fn with_key(mut self) -> Self {
        self.uuid = Some(Uuid::new_v4());

        self
    }

    /// Inserts the given data into the identifier.
    #[must_use]
    pub fn with(mut self, data: impl Into<String>) -> Self {
        self.data.push(data.into().into_boxed_str());

        self
    }

    /// Validates the length of this [`CId`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the identifier is too long.
    pub fn validate(self) -> Result<Self> {
        let string = self.to_string();

        // currently, afaik, this is the only check we need; from what i can find custom identifiers
        // are not limited by a specific charset, only by length.
        if string.len() >= Self::MAX_LEN {
            bail!("maximum identifier length exceeded ({}/{} bytes)", string.len(), Self::MAX_LEN);
        }

        Ok(self)
    }
}

impl TryFrom<&String> for CId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(&(**value))
    }
}

impl TryFrom<String> for CId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(&(*value))
    }
}

impl TryFrom<&Box<str>> for CId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &Box<str>) -> Result<Self, Self::Error> {
        Self::try_from(&(**value))
    }
}

impl TryFrom<Box<str>> for CId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
        Self::try_from(&(*value))
    }
}

impl TryFrom<Cow<'_, str>> for CId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: Cow<str>) -> Result<Self, Self::Error> {
        Self::try_from(&(*value))
    }
}

impl TryFrom<&str> for CId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl FromStr for CId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(Self::PART_SEP).take(4);

        // every valid CID must have a handler name and component kind identifier.
        let Some(name) = parts.next() else {
            bail!("missing event handler name");
        };
        let Some(kind) = parts.next() else {
            bail!("missing component kind");
        };

        let mut cid = Self::new(name, kind);

        // this will only run zero, one, or two times.
        for part in parts {
            // we prefix the storage key with "K_" to *try* not to read data that contains a UUID as
            // the storage key identifier.
            if part.starts_with("K_") {
                cid.uuid = Some(part.trim_start_matches("K_").parse()?);
            } else {
                cid.data = part.split(Self::DATA_SEP).map(Into::into).collect();
            }
        }

        Ok(cid)
    }
}

impl From<CId> for String {
    #[inline]
    fn from(value: CId) -> Self {
        value.to_string()
    }
}

impl From<CId> for Box<str> {
    #[inline]
    fn from(value: CId) -> Self {
        value.to_string().into_boxed_str()
    }
}

impl Display for CId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { name: (name, kind), data, uuid } = self;

        write!(f, "{name}{}{kind}", Self::PART_SEP)?;
        // only write the UUID if it exists; shorthand for an if-let-some statement.
        uuid.map_or(Ok(()), |uuid| write!(f, "{}K_{uuid}", Self::PART_SEP))?;

        if data.is_empty() {
            Ok(())
        } else {
            // write all stringified internal data joined by the data separator character.
            write!(f, "{}{}", Self::PART_SEP, data.join(&Self::DATA_SEP.to_string()))
        }
    }
}

/// Resolves and tracks a command's provided options.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandOptionResolver<'evt> {
    /// The inner command data.
    data: &'evt CommandData,
    /// The inner map of options and their values.
    options: HashMap<&'evt str, &'evt CommandOptionValue>,
}

impl<'evt> CommandOptionResolver<'evt> {
    /// Creates a new [`CommandOptionResolver`] with the given data and options.
    #[inline]
    #[must_use]
    fn new_from(data: &'evt CommandData, options: &'evt [CommandDataOption]) -> Self {
        Self { data, options: options.iter().map(|o| (&(*o.name), &o.value)).collect() }
    }

    /// Creates a new [`CommandOptionResolver`] with the given data.
    #[inline]
    #[must_use]
    pub fn new(data: &'evt CommandData) -> Self {
        Self::new_from(data, &data.options)
    }

    /// Returns a reference to a stored [`CommandOptionValue`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the requested option does not exist.
    fn get(&self, name: &str) -> Result<&CommandOptionValue> {
        let Some(value) = self.options.get(name) else {
            bail!("missing value for option '{name}'");
        };

        Ok(*value)
    }

    /// Returns a new [`CommandOptionResolver`] containing a sub-command's options.
    ///
    /// # Errors
    ///
    /// This function will return an error if the sub-command does not exist or the value associated
    /// with the given option name is an invalid type.
    pub fn get_subcommand(&'evt self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommand(ref value) = self.get(name)? else {
            bail!("invalid type for option '{name}'");
        };

        Ok(Self::new_from(self.data, value))
    }

    /// Returns a new [`CommandOptionResolver`] containing a sub-command group's options.
    ///
    /// # Errors
    ///
    /// This function will return an error if the sub-command group does not exist or the value
    /// associated with the given option name is an invalid type.
    pub fn get_subcommand_group(&'evt self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommandGroup(ref value) = self.get(name)? else {
            bail!("invalid type for option '{name}'");
        };

        Ok(Self::new_from(self.data, value))
    }
}

/// Generates getter methods for the [`CommandOptionResolver`] struct.
///
/// # Examples
///
/// ```
/// command_option_resolver_getter! {
///     /// Gets a boolean.
///     fn get_bool() -> Boolean as bool;
/// }
/// ```
macro_rules! command_option_resolver_getter {
    ($(
        $(#[$attribute:meta])*
        fn $name:ident() -> $variant:ident as $return:ty;
    )*) => {
        impl<'evt> CommandOptionResolver<'evt> {$(
            $(#[$attribute])*
            pub fn $name(&'evt self, name: &str) -> Result<&'evt $return> {
                let CommandOptionValue::$variant(ref value) = self.get(name)? else {
                    bail!("invalid type for option '{name}'");
                };

                Ok(value)
            }
        )*}
    };
}

command_option_resolver_getter! {
    /// Returns a reference to a stored [`Id<AttachmentMarker>`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_attachment_id() -> Attachment as Id<AttachmentMarker>;

    /// Returns a reference to a stored [`bool`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_bool() -> Boolean as bool;

    /// Returns a reference to a stored [`Id<ChannelMarker>`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_channel_id() -> Channel as Id<ChannelMarker>;

    /// Returns a reference to a stored [`i64`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_i64() -> Integer as i64;

    /// Returns a reference to a stored [`Id<GenericMarker>`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_mentionable_id() -> Mentionable as Id<GenericMarker>;

    /// Returns a reference to a stored [`f64`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_f64() -> Number as f64;

    /// Returns a reference to a stored [`Id<RoleMarker>`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_role_id() -> Role as Id<RoleMarker>;

    /// Returns a reference to a stored [`str`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_str() -> String as str;

    /// Returns a reference to a stored [`Id<UserMarker>`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist or the value associated with
    /// the given option name is an invalid type.
    fn get_user_id() -> User as Id<UserMarker>;
}
