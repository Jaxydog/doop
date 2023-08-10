use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, bail};
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
use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, GenericMarker, GuildMarker, RoleMarker, UserMarker,
};
use twilight_model::id::Id;
use uuid::Uuid;

use crate::util::Result;

/// A command interaction context.
pub type CommandCtx<'b> = Ctx<'b, &'b CommandData>;
/// A component interaction context.
pub type ComponentCtx<'b> = Ctx<'b, &'b MessageComponentInteractionData>;
/// A modal interaction context.
pub type ModalCtx<'b> = Ctx<'b, &'b ModalInteractionData>;

/// A basic event handler.
pub trait EventHandler: Send + Sync {
    /// The name of this [`EventHandler`].
    fn name(&self) -> &'static str;
}

/// An interaction event handler.
#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait InteractionEventHandler: CommandBuilder + EventHandler {
    /// Handles a autocomplete interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the autocomplete could not be handled.
    async fn handle_autocomplete<'b>(
        &self,
        ctx: CommandCtx<'b>,
        focus: (&'b str, CommandOptionType),
    ) -> Result<Vec<CommandOptionChoice>> {
        bail!("unimplemented interaction type");
    }

    /// Handles a command interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the command could not be handled.
    async fn handle_command(&self, ctx: CommandCtx<'_>) -> Result {
        bail!("unimplemented interaction type");
    }

    /// Handles a component interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the component could not be handled.
    async fn handle_component(&self, ctx: ComponentCtx<'_>, data: CustomData) -> Result {
        bail!("unimplemented interaction type");
    }

    /// Handles a modal interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the modal could not be handled.
    async fn handle_modal(&self, ctx: ModalCtx<'_>, data: CustomData) -> Result {
        bail!("unimplemented interaction type");
    }
}

/// A value that can create a [`Command`].
pub trait CommandBuilder {
    /// Builds the [`Command`] of this [`EventHandler`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the command could not be built.
    fn command(&self, guild_id: Option<Id<GuildMarker>>) -> Result<Option<Command>>;
}

/// A reference to the bot's HTTP API and cache instance.
#[derive(Clone, Copy, Debug)]
pub struct Api<'b> {
    /// The API's HTTP value.
    http: &'b Arc<Client>,
    /// The API's cache value.
    cache: &'b Arc<InMemoryCache>,
}

impl<'b> Api<'b> {
    /// Creates a new [`Api`].
    pub const fn new(http: &'b Arc<Client>, cache: &'b Arc<InMemoryCache>) -> Self {
        Self { http, cache }
    }

    /// Returns a reference to the HTTP client of this [`Api`].
    #[must_use]
    pub const fn http(&self) -> &'b Arc<Client> { self.http }

    /// Returns a reference to the cache of this [`Api`].
    #[must_use]
    pub const fn cache(&self) -> &'b Arc<InMemoryCache> { self.cache }
}

impl<'b> From<(&'b Arc<Client>, &'b Arc<InMemoryCache>)> for Api<'b> {
    #[inline]
    fn from((http, cache): (&'b Arc<Client>, &'b Arc<InMemoryCache>)) -> Self {
        Self { http, cache }
    }
}

impl<'b> From<(&'b Arc<InMemoryCache>, &'b Arc<Client>)> for Api<'b> {
    #[inline]
    fn from((cache, http): (&'b Arc<InMemoryCache>, &'b Arc<Client>)) -> Self {
        Self { http, cache }
    }
}

/// An interaction event context.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'b, T> {
    /// The context's HTTP API and cache.
    pub api: Api<'b>,
    /// The context's interaction event.
    pub event: &'b Interaction,
    /// The context's stored data.
    pub data: T,
}

impl<'b, T> Ctx<'b, T> {
    /// Creates a new [`Ctx<T>`].
    pub const fn new(api: Api<'b>, event: &'b Interaction, data: T) -> Self {
        Self { api, event, data }
    }

    /// Returns the interaction client of this [`Ctx<T>`].
    pub fn client(&self) -> InteractionClient {
        self.api.http().interaction(self.event.application_id)
    }
}

/// Data stored within a component or modal's `custom_id`.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomData {
    /// The source of the component.
    source: (Box<str>, Box<str>),
    /// The internal stringified data.
    data: Vec<Box<str>>,
    /// The storage key identifier.
    uuid: Option<Uuid>,
}

impl CustomData {
    /// The maximum length of an identifier in bytes.
    pub const MAX_LEN: usize = 100;

    /// Creates a new [`CustomData`].
    pub fn new(handler: impl AsRef<str>, component: impl AsRef<str>) -> Self {
        let name = handler.as_ref().into();
        let kind = component.as_ref().into();

        Self { source: (name, kind), data: vec![], uuid: None }
    }

    /// Validates this [`CustomData`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the identifier is too long.
    pub fn validate(self) -> Result<Self> {
        if self.to_string().len() > Self::MAX_LEN {
            bail!("maximum identifier length exceeded (> {} bytes)", Self::MAX_LEN);
        }

        Ok(self)
    }

    /// Generates a new storage key for this [`CustomData`].
    #[inline]
    pub fn generate_key(&mut self) { self.uuid = Some(Uuid::new_v4()) }

    /// Inserts the given data into this [`CustomData`].
    #[inline]
    pub fn insert(&mut self, data: impl AsRef<str>) { self.data.push(data.as_ref().into()); }

    /// Inserts the given data into this [`CustomData`].
    #[inline]
    #[must_use]
    pub fn with(mut self, data: impl AsRef<str>) -> Self {
        self.insert(data);
        self
    }

    /// Returns a reference to the handler name of this [`CustomData`].
    #[inline]
    #[must_use]
    pub const fn handler_name(&self) -> &str { &self.source.0 }

    /// Returns a reference to the component name of this [`CustomData`].
    #[inline]
    #[must_use]
    pub const fn component_name(&self) -> &str { &self.source.1 }

    /// Returns a reference to the data of this [`CustomData`].
    #[inline]
    #[must_use]
    pub fn data(&self) -> &[Box<str>] { &self.data }

    /// Returns the storage key of this [`CustomData`].
    #[inline]
    #[must_use]
    pub fn key<T>(&self) -> Option<FileKey<T, Compress<MsgPack, 3>>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        Some(format!(".persist/{}/{}/{}", self.source.0, self.source.1, self.uuid?).into())
    }
}

impl TryFrom<String> for CustomData {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> { Self::from_str(&value) }
}

impl TryFrom<&String> for CustomData {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> { Self::from_str(value) }
}

impl TryFrom<Box<str>> for CustomData {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: Box<str>) -> Result<Self, Self::Error> { Self::from_str(&value) }
}

impl TryFrom<&Box<str>> for CustomData {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &Box<str>) -> Result<Self, Self::Error> { Self::from_str(value) }
}

impl TryFrom<&str> for CustomData {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> { Self::from_str(value) }
}

impl FromStr for CustomData {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('$').take(4);

        let Some(name) = parts.next() else {
            bail!("missing event handler name");
        };
        let Some(kind) = parts.next() else {
            bail!("missing component name");
        };

        let mut id = Self::new(name, kind);

        for part in parts {
            if let Ok(uuid) = part.parse() {
                id.uuid = Some(uuid);
            } else {
                id.data = part.split(';').map(Into::into).collect();
                break;
            }
        }

        Ok(id)
    }
}

impl From<CustomData> for String {
    #[inline]
    fn from(value: CustomData) -> Self { value.to_string() }
}

impl Display for CustomData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { source: (name, kind), data, uuid } = self;

        write!(f, "{name}${kind}")?;
        uuid.map_or(Ok(()), |uuid| write!(f, "${uuid}"))?;

        if !data.is_empty() {
            write!(f, "${}", data.join(";"))?;
        }

        Ok(())
    }
}

/// Resolves and keep track of a command's options.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandOptionResolver<'c> {
    /// The inner command data.
    data: &'c CommandData,
    /// The inner option map.
    options: HashMap<&'c str, &'c CommandOptionValue>,
}

impl<'c> CommandOptionResolver<'c> {
    /// Creates a new [`CommandOptionResolver`] with the given options.
    #[inline]
    #[must_use]
    pub fn new(data: &'c CommandData) -> Self { Self::new_from(data, &data.options) }

    /// Creates a new [`CommandOptionResolver`] with the given options.
    #[inline]
    #[must_use]
    fn new_from(data: &'c CommandData, options: &'c [CommandDataOption]) -> Self {
        Self { data, options: options.iter().map(|o| (&(*o.name), &o.value)).collect() }
    }

    /// Returns a reference to a stored [`CommandOptionValue`] with the given name.
    ///
    /// # Errors
    ///
    /// This function will return an error if the option does not exist.
    fn get(&self, name: &str) -> Result<&CommandOptionValue> {
        let Some(value) = self.options.get(name) else {
            return Err(anyhow!("missing value for option '{name}'"));
        };

        Ok(*value)
    }

    /// Returns a new [`CommandOptionResolver`] containing a sub-command's options.
    ///
    /// # Errors
    ///
    /// This function will return an error if the sub-command does not exist or the value associated
    /// with the given option name is an invalid type.
    pub fn get_subcommand(&'c self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommand(ref options) = self.get(name)? else {
            bail!("invalid type for option '{name}'");
        };

        Ok(Self::new_from(self.data, options))
    }

    /// Returns a new [`CommandOptionResolver`] containing a sub-command group's options.
    ///
    /// # Errors
    ///
    /// This function will return an error if the sub-command group does not exist or the value
    /// associated with the given option name is an invalid type.
    pub fn get_subcommand_group(&'c self, name: &str) -> Result<Self> {
        let CommandOptionValue::SubCommandGroup(ref options) = self.get(name)? else {
            bail!("invalid type for option '{name}'");
        };

        Ok(Self::new_from(self.data, options))
    }
}

/// Generates getter methods for the [`CommandOptionResolver`] type.
///
/// ```
/// cor_getter! {
///     /// Getter method.
///     fn get_bool() -> Boolean as bool;
/// }
/// ```
macro_rules! cor_getter {
    {$(
        $(#[$attribute:meta])*
        fn $name:ident() -> $variant:ident as $return:ty;
    )*} => {
        impl<'c> CommandOptionResolver<'c> {$(
            $(#[$attribute])*
            #[inline]
            pub fn $name(&'c self, name: &str) -> Result<&'c $return> {
                let CommandOptionValue::$variant(ref value) = self.get(name)? else {
                    bail!("invalid type for option '{name}'");
                };

                Ok(value)
            }
        )*}
    };
}

cor_getter! {
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
