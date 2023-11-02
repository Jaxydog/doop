use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use anyhow::bail;
use doop_logger::warn;
use twilight_model::application::command::{Command, CommandOptionChoice, CommandOptionType};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, GenericMarker, GuildMarker, RoleMarker, UserMarker,
};
use twilight_model::id::Id;

use crate::bot::interaction::{CommandCtx, ComponentCtx, ModalCtx};
use crate::util::{DataId, Result};

/// The help command.
pub mod help;
/// The ping command.
pub mod ping;

/// The bot's command registry.
static REGISTRY: OnceLock<CommandRegistry> = OnceLock::new();

/// Initializes the command registry.
macro_rules! init_registry {
    ($($init:expr),* $(,)?) => {
        /// Returns a reference to the bot's command registry.
        pub fn registry() -> &'static CommandRegistry {
            REGISTRY.get_or_init(|| {
                let mut registry = CommandRegistry::new();

                $({
                    let entry = $init();

                    if !registry.register(entry) {
                        ::doop_logger::warn!("the '{}' command has already been registered", entry.name).ok();
                    }
                })*

                registry
            })
        }
    };
}

init_registry![self::help::entry, self::ping::entry];

/// A builder function.
pub type BuildFn = fn(&CommandEntry, Option<Id<GuildMarker>>) -> Result<Option<Command>>;

/// Maintains a list of registered commands and their associated interaction handlers.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    /// The inner set of commands.
    inner: HashSet<CommandEntry>,
}

impl CommandRegistry {
    /// Creates a new [`CommandRegistry`].
    #[must_use]
    pub fn new() -> Self {
        Self { inner: HashSet::new() }
    }

    /// Returns the command entry with the given name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CommandEntry> {
        self.inner.iter().find(|e| e.name == name)
    }

    /// Returns an iterator over this [`CommandRegistry`].
    pub fn iter(&self) -> impl Iterator<Item = &CommandEntry> {
        self.inner.iter()
    }

    /// Builds all registered commands.
    #[must_use]
    pub fn build_all(&self, guild_id: Option<Id<GuildMarker>>) -> Box<[Command]> {
        let commands = self.inner.iter().filter_map(|e| match e.build(guild_id) {
            Ok(command) => command,
            Err(error) => {
                warn!("the '{}' command failed to build - {error}", e.name).ok();
                None
            }
        });

        commands.collect()
    }

    /// Registers the given command entry, returning whether it was successfully registered.
    #[inline]
    pub fn register(&mut self, entry: CommandEntry) -> bool {
        self.inner.insert(entry)
    }
}

/// An entry within the command registry.
#[derive(Clone, Copy, Debug)]
pub struct CommandEntry {
    /// The command's name.
    pub name: &'static str,
    /// Constructs a Discord command from the entry.
    builder: BuildFn,
    /// A list of getters for the command's interaction event handlers.
    handlers: CommandEntryHandlers,
}

impl CommandEntry {
    /// Creates a new [`CommandEntry`].
    pub const fn new(name: &'static str, builder: BuildFn, handlers: CommandEntryHandlers) -> Self {
        Self { name, builder, handlers }
    }

    /// Returns a constructed Discord command from this [`CommandEntry`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the command could not be constructed.
    #[inline]
    pub fn build(&self, guild_id: Option<Id<GuildMarker>>) -> Result<Option<Command>> {
        (self.builder)(self, guild_id)
    }

    /// Returns the command handler of this [`CommandEntry`].
    #[must_use]
    pub fn command(&self) -> Option<Box<dyn OnCommand + Send + Sync>> {
        self.handlers.command.map(|f| f(self))
    }

    /// Returns the auto-completion handler of this [`CommandEntry`].
    #[must_use]
    pub fn complete(&self) -> Option<Box<dyn OnComplete + Send + Sync>> {
        self.handlers.complete.map(|f| f(self))
    }

    /// Returns the component handler of this [`CommandEntry`].
    #[must_use]
    pub fn component(&self) -> Option<Box<dyn OnComponent + Send + Sync>> {
        self.handlers.component.map(|f| f(self))
    }

    /// Returns the modal handler of this [`CommandEntry`].
    #[must_use]
    pub fn modal(&self) -> Option<Box<dyn OnModal + Send + Sync>> {
        self.handlers.modal.map(|f| f(self))
    }
}

impl PartialEq for CommandEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CommandEntry {}

impl Hash for CommandEntry {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

/// Maintains a list of getters for a command's interaction event handlers.
#[allow(clippy::type_complexity)] // handled by a macro.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandEntryHandlers {
    /// Returns a command interaction event handler.
    pub command: Option<fn(&CommandEntry) -> Box<dyn OnCommand + Send + Sync>>,
    /// Returns an auto-completion interaction event handler.
    pub complete: Option<fn(&CommandEntry) -> Box<dyn OnComplete + Send + Sync>>,
    /// Returns a component interaction event handler.
    pub component: Option<fn(&CommandEntry) -> Box<dyn OnComponent + Send + Sync>>,
    /// Returns a modal interaction event handler.
    pub modal: Option<fn(&CommandEntry) -> Box<dyn OnModal + Send + Sync>>,
}

impl CommandEntryHandlers {
    /// Creates a new [`CommandEntryHandlers`].
    #[must_use]
    pub const fn new() -> Self {
        Self { command: None, complete: None, component: None, modal: None }
    }
}

/// Handles a command interaction event.
#[async_trait::async_trait]
pub trait OnCommand {
    /// Returns a reference to the source command entry.
    fn entry(&self) -> &CommandEntry;

    /// Responds to a command interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if event handling failed.
    async fn execute<'api: 'evt, 'evt>(&self, ctx: CommandCtx<'api, 'evt>) -> Result;
}

/// Handles an auto-completion interaction event.
#[async_trait::async_trait]
pub trait OnComplete {
    /// Returns a reference to the source command entry.
    fn entry(&self) -> &CommandEntry;

    /// Responds to an auto-completion interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if event handling failed.
    async fn execute<'api: 'evt, 'evt>(
        &self,
        ctx: CommandCtx<'api, 'evt>,
        focus: (&'evt str, CommandOptionType),
    ) -> Result<Vec<CommandOptionChoice>>;
}

/// Handles a component interaction event.
#[async_trait::async_trait]
pub trait OnComponent {
    /// Returns a reference to the source command entry.
    fn entry(&self) -> &CommandEntry;

    /// Responds to a component interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if event handling failed.
    async fn execute<'api: 'evt, 'evt>(&self, ctx: ComponentCtx<'api, 'evt>, id: DataId) -> Result;
}

/// Handles a modal interaction event.
#[async_trait::async_trait]
pub trait OnModal {
    /// Returns a reference to the source command entry.
    fn entry(&self) -> &CommandEntry;

    /// Responds to a modal interaction event.
    ///
    /// # Errors
    ///
    /// This function will return an error if event handling failed.
    async fn execute<'api: 'evt, 'evt>(&self, ctx: ModalCtx<'api, 'evt>, id: DataId) -> Result;
}

/// Creates a command registry entry.
///
/// ```
/// register_command! {
///     ChatInput("test") {
///         let in_dms = true;
///         let is_nsfw = false;
///         let require = ADMINISTRATOR;
///         let options = [
///             Integer("int") {
///                 let required = false;
///                 let autocomplete = true;
///                 let minimum = 0;
///                 let maximum = 2048;
///                 let choices = [("one", 1), ("two", 2), ("three", 3)];
///             },
///             String("test") {},
///         ];
///         let handlers = {
///             command = self::execute_command;
///             complete = self::execute_complete;
///             component = self::execute_component;
///             modal = self::execute_modal;
///         };
///     }
/// }
/// ```
#[macro_export]
macro_rules! register_command {
    {
        $(#[developer($developer:literal)])?
        $kind:ident($name:literal) {
            $(let in_dms = $dms:literal;)?
            $(let is_nsfw = $nsfw:literal;)?
            $(let require = $($permission:ident)|+;)?
            $(let options = [$($option_kind:ident($option_name:literal) {$($args:tt)*}),* $(,)?];)?
            $(let handlers = {
                $(command = $command:expr;)?
                $(complete = $complete:expr;)?
                $(component = $component:expr;)?
                $(modal = $modal:expr;)?
            };)?
        }
    } => {
        /// Returns this command's entry.
        pub fn entry() -> $crate::cmd::CommandEntry {
            fn build(
                entry: &$crate::cmd::CommandEntry,
                guild_id: ::std::option::Option<::twilight_model::id::Id<::twilight_model::id::marker::GuildMarker>>
            ) -> $crate::util::Result<::std::option::Option<::twilight_model::application::command::Command>> {
                $(if $developer && guild_id.is_none() {
                    return ::std::result::Result::Ok(::std::option::Option::None);
                })?

                let mut builder = ::twilight_util::builder::command::CommandBuilder::new(
                        entry.name,
                        ::doop_localizer::localize!("command.{}.description", entry.name),
                        ::twilight_model::application::command::CommandType::$kind,
                    )
                    .name_localizations(::doop_localizer::localize!(in *, "command.{}.name", entry.name))
                    .description_localizations(::doop_localizer::localize!(in *, "command.{}.description", entry.name))
                    $(.default_member_permissions(::twilight_model::guild::Permissions::empty()$(.union(::twilight_model::guild::Permissions::$permission))+))?
                    $(.dm_permission($dms))?
                    $(.nsfw($nsfw))?
                    $($(.option($crate::register_command!(@option(entry, $option_kind($option_name) { $($args)* }))))*)?;

                if let ::std::option::Option::Some(id) = guild_id {
                    builder = builder.guild_id(id);
                }

                ::std::result::Result::Ok(::std::option::Option::Some(builder.validate()?.build()))
            }

            #[allow(unused_mut)]
            let mut handlers = $crate::cmd::CommandEntryHandlers::new();

            $(
                $(handlers.command = {
                    struct Struct($crate::cmd::CommandEntry);

                    #[::async_trait::async_trait]
                    impl $crate::cmd::OnCommand for Struct {
                        fn entry(&self) -> &$crate::cmd::CommandEntry { &self.0 }

                        #[inline]
                        async fn execute<'api: 'evt, 'evt>(&self, ctx: $crate::bot::interaction::CommandCtx<'api, 'evt>) -> $crate::util::Result {
                            $command(self, ctx).await
                        }
                    }

                    Some(|e| ::std::boxed::Box::new(Struct(*e)))
                };)?
                $(handlers.complete = {
                    struct Struct($crate::cmd::CommandEntry);

                    #[::async_trait::async_trait]
                    impl $crate::cmd::OnComplete for Struct {
                        fn entry(&self) -> &$crate::cmd::CommandEntry { &self.0 }

                        #[inline]
                        async fn execute<'api: 'evt, 'evt>(
                            &self,
                            ctx: $crate::bot::interaction::CommandCtx<'api, 'evt>,
                            focus: (&'evt str, CommandOptionType)
                        ) -> $crate::util::Result<::std::vec::Vec<::twilight_model::application::command::CommandOptionChoice>> {
                            $complete(self, ctx, focus).await
                        }
                    }

                    Some(|e| ::std::boxed::Box::new(Struct(*e)))
                };)?
                $(handlers.component = {
                    struct Struct($crate::cmd::CommandEntry);

                    #[::async_trait::async_trait]
                    impl $crate::cmd::OnComponent for Struct {
                        fn entry(&self) -> &$crate::cmd::CommandEntry { &self.0 }

                        #[inline]
                        async fn execute<'api: 'evt, 'evt>(&self, ctx: $crate::bot::interaction::ComponentCtx<'api, 'evt>, id: $crate::util::DataId) -> $crate::util::Result {
                            $component(self, ctx, id).await
                        }
                    }

                    Some(|e| ::std::boxed::Box::new(Struct(*e)))
                };)?
                $(handlers.modal = {
                    struct Struct($crate::cmd::CommandEntry);

                    #[::async_trait::async_trait]
                    impl $crate::cmd::OnModal for Struct {
                        fn entry(&self) -> &$crate::cmd::CommandEntry { &self.0 }

                        #[inline]
                        async fn execute<'api: 'evt, 'evt>(&self, ctx: $crate::bot::interaction::ModalCtx<'api, 'evt>, id: $crate::util::DataId) -> $crate::util::Result {
                            $modal(self, ctx, id).await
                        }
                    }

                    Some(|e| ::std::boxed::Box::new(Struct(*e)))
                };)?
            )?

            $crate::cmd::CommandEntry::new($name, build, handlers)
        }
    };
    (@option($entry:expr, Attachment($name:literal) {
        $(let required = $required:literal;)?
    })) => {{
        ::twilight_util::builder::command::AttachmentBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
    }};
    (@option($entry:expr, Boolean($name:literal) {
        $(let required = $required:literal;)?
    })) => {{
        ::twilight_util::builder::command::BooleanBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
    }};
    (@option($entry:expr, Channel($name:literal) {
        $(let required = $required:literal;)?
        $(let channels = $($channel:ident)|+;)?
    })) => {{
        ::twilight_util::builder::command::ChannelBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
            $(.channel_types([$(::twilight_model::channel::ChannelType::$channel),+]))?
    }};
    (@option($entry:expr, Integer($name:literal) {
        $(let required = $required:literal;)?
        $(let autocomplete = $autocomplete:literal;)?
        $(let minimum = $minimum:literal;)?
        $(let maximum = $maximum:literal;)?
        $(let choices = [$(($choice:literal, $value:expr)),+ $(,)?];)?
    })) => {{
        ::twilight_util::builder::command::IntegerBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
            $(.autocomplete($autocomplete))?
            $(.min_value($minimum))?
            $(.max_value($maximum))?
            $(
                .choices(vec![$(($choice, $value)),*])
                $(.choice_localizations($choice, ::doop_localizer::localize!(in *, "option.{}.{}.choice.{}", $entry.name, $name, $choice)))*
            )?
    }};
    (@option($entry:expr, Mentionable($name:literal) {
        $(let required = $required:literal;)?
    })) => {{
        ::twilight_util::builder::command::MentionableBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
    }};
    (@option($entry:expr, Number($name:literal) {
        $(let required = $required:literal;)?
        $(let autocomplete = $autocomplete:literal;)?
        $(let minimum = $minimum:literal;)?
        $(let maximum = $maximum:literal;)?
        $(let choices = [$(($choice:literal, $value:expr)),+ $(,)?];)?
    })) => {{
        ::twilight_util::builder::command::NumberBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
            $(.autocomplete($autocomplete))?
            $(.min_value($minimum))?
            $(.max_value($maximum))?
            $(
                .choices(vec![$(($choice, $value)),*])
                $(.choice_localizations($choice, ::doop_localizer::localize!(in *, "option.{}.{}.choice.{}", $entry.name, $name, $choice)))*
            )?
    }};
    (@option($entry:expr, Role($name:literal) {
        $(let required = $required:literal;)?
    })) => {{
        ::twilight_util::builder::command::RoleBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
    }};
    (@option($entry:expr, String($name:literal) {
        $(let required = $required:literal;)?
        $(let autocomplete = $autocomplete:literal;)?
        $(let minimum = $minimum:literal;)?
        $(let maximum = $maximum:literal;)?
        $(let choices = [$(($choice:literal, $value:expr)),+ $(,)?];)?
    })) => {{
        ::twilight_util::builder::command::StringBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
            $(.autocomplete($autocomplete))?
            $(.min_length($minimum))?
            $(.max_length($maximum))?
            $(
                .choices(vec![$(($choice, $value)),*])
                $(.choice_localizations($choice, ::doop_localizer::localize!(in *, "option.{}.{}.choice.{}", $entry.name, $name, $choice)))*
            )?
    }};
    (@option($entry:expr, SubCommand($name:literal) {
        $(let required = $required:literal;)?
        $(let options = [$($option_kind:ident($option_name:literal) {$($args:tt)*}),* $(,)?];)?
    })) => {{
        ::twilight_util::builder::command::SubCommandBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $($(.option($crate::register_command!(@option(entry, $option_kind($option_name) { $($args)* }))))*)?;
    }};
    (@option($entry:expr, SubCommandGroup($name:literal) {
        $(let required = $required:literal;)?
        $(let commands = [$($option_kind:ident($option_name:literal) {$($args:tt)*}),* $(,)?];)?
    })) => {{
        ::twilight_util::builder::command::SubCommandGroupBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.subcommands(vec![$($crate::register_command!(@option(entry, $option_kind($option_name) { $($args)* }))),*]))?
    }};
    (@option($entry:expr, User($name:literal) {
        $(let required = $required:literal;)?
    })) => {{
        ::twilight_util::builder::command::UserBuilder::new($name, ::doop_localizer::localize!("option.{}.{}.description", $entry.name, $name))
            .name_localizations(::doop_localizer::localize!(in *, "option.{}.{}.name", $entry.name, $name))
            .description_localizations(::doop_localizer::localize!(in *, "option.{}.{}.description", $entry.name, $name))
            $(.required($required))?
    }};
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
