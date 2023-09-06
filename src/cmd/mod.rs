/// The embed command.
pub mod embed;
/// The help command.
pub mod help;
/// The ping command.
pub mod ping;
/// The role command.
pub mod role;

/// Defines a command and event handler.
///
/// # Examples
///
/// ```
/// command! {
///     #![developer(false)]
///
///     let name = "do_something";
///     let kind = ChatInput;
///     let permissions = USE_SLASH_COMMANDS;
///     let allow_dms = true;
///     let is_nsfw = false;
///     let options = [
///         {
///             let name = "boolean";
///             let kind = Boolean;
///             let required = false;
///         },
///         {
///             let name = "subcommand";
///             let kind = SubCommand;
///             let options = [
///                 {
///                     let name = "attachment";
///                     let kind = Attachment;
///                     let required = false;
///                 },
///                 {
///                     let name = "another_boolean";
///                     let kind = Boolean;
///                     let required = false;
///                 },
///             ];
///         }
///     ];
/// }
/// ```
#[macro_export]
macro_rules! command {
    {
        $(#![developer($developer:literal)])?

        let name = $name:literal;
        let kind = $kind:ident;
        $(let permissions = $($permission:ident)|+;)?
        $(let allow_dms = $allow_dms:literal;)?
        $(let is_nsfw = $is_nsfw:literal;)?
        $(let options = [$({$($args:tt)+}),* $(,)?];)?
    } => {
        /// A command implementation.
        #[derive(::std::fmt::Debug, ::std::default::Default)]
        pub struct Impl;

        impl Impl {
            /// The command name.
            pub const NAME: &'static str = $name;
            /// The command type.
            pub const KIND: ::twilight_model::application::command::CommandType = ::twilight_model::application::command::CommandType::$kind;
        }

        impl $crate::bot::interact::Handler for Impl {
            fn name(&self) -> &'static str { Self::NAME }
        }

        impl $crate::bot::interact::AsCommand for Impl {
            fn command(
                &self,
                guild_id: ::std::option::Option<::twilight_model::id::Id<::twilight_model::id::marker::GuildMarker>>
            ) -> $crate::util::Result<::std::option::Option<::twilight_model::application::command::Command>> {
                $(if $developer && guild_id.is_none() { return Ok(None); })?

                let description = ::doop_localizer::localize!("command.{}.description", Self::NAME);
                let name_map = ::doop_localizer::localize_map!("command.{}.name", Self::NAME);
                let description_map = ::doop_localizer::localize_map!("command.{}.description", Self::NAME);
                let mut builder = ::twilight_util::builder::command::CommandBuilder::new(Self::NAME, description, Self::KIND)
                    .name_localizations(name_map)
                    .description_localizations(description_map)
                    $(.dm_permission($allow_dms))?
                    $(.nsfw($is_nsfw))?
                    $($(.option($crate::command!(@option({$($args)+}))))*)?;

                if let Some(id) = guild_id { builder = builder.guild_id(id); }

                $(
                    let permissions = ::twilight_model::guild::Permissions::empty()
                        $(.union(::twilight_model::guild::Permissions::$permission))+;

                    builder = builder.default_member_permissions(permissions);
                )?

                Ok(Some(builder.validate()?.build()))
            }
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Attachment;
        let required = $required:literal;
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::AttachmentBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Boolean;
        let required = $required:literal;
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::BooleanBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Channel;
        let required = $required:literal;
        $( let kinds = $( $kind:ident )|+; )?
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::ChannelBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
                $(.channel_types([$(::twilight_model::channel::ChannelType::$kind),+]))?
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Integer;
        let required = $required:literal;
        $(let autocomplete = $autocomplete:literal;)?
        $(let min = $min:literal;)?
        $(let max = $max:literal;)?
        $(let choices = [$(($choice:literal, $value:expr)),* $(,)?];)?
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);
            #[allow(unused_mut)]
            let mut builder = ::twilight_util::builder::command::IntegerBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
                $(.autocomplete($autocomplete))?
                $(.min_value($min))?
                $(.max_value($max))?;

            $(
                let choices: &[(&str, _)] = &[$(($choice, $value)),+];

                builder = builder.choices(choices.to_vec());

                for (choice, _) in choices {
                    let localized = ::doop_localizer::localize_map!("option.{}.{}.choice.{choice}", Self::NAME, $name);

                    builder = builder.choice_localizations(choice, localized);
                }
            )?

            builder
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Mentionable;
        let required = $required:literal;
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::MentionableBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Number;
        let required = $required:literal;
        $(let autocomplete = $autocomplete:literal;)?
        $(let min = $min:literal;)?
        $(let max = $max:literal;)?
        $(let choices = [$(($choice:literal, $value:expr)),* $(,)?];)?
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);
            #[allow(unused_mut)]
            let mut builder = ::twilight_util::builder::command::NumberBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
                $(.autocomplete($autocomplete))?
                $(.min_value($min))?
                $(.max_value($max))?;

            $(
                let choices: &[(&str, _)] = &[$(($choice, $value)),+];

                builder = builder.choices(choices.to_vec());

                for (choice, _) in choices {
                    let localized = ::doop_localizer::localize_map!("option.{}.{}.choice.{choice}", Self::NAME, $name);

                    builder = builder.choice_localizations(choice, localized);
                }
            )?

            builder
        }
    };
    (@option({
        let name = $name:literal;
        let kind = Role;
        let required = $required:literal;
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::RoleBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
        }
    };
    (@option({
        let name = $name:literal;
        let kind = String;
        let required = $required:literal;
        $(let autocomplete = $autocomplete:literal;)?
        $(let min = $min:literal;)?
        $(let max = $max:literal;)?
        $(let choices = [$(($choice:literal, $value:expr)),* $(,)?];)?
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);
            #[allow(unused_mut)]
            let mut builder = ::twilight_util::builder::command::StringBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
                $(.autocomplete($autocomplete))?
                $(.min_length($min))?
                $(.max_length($max))?;

            $(
                let choices: &[(&str, _)] = &[$(($choice, $value)),+];

                builder = builder.choices(choices.to_vec());

                for (choice, _) in choices {
                    let localized = ::doop_localizer::localize_map!("option.{}.{}.choice.{choice}", Self::NAME, $name);

                    builder = builder.choice_localizations(choice, localized);
                }
            )?

            builder
        }
    };
    (@option({
        let name = $name:literal;
        let kind = SubCommand;
        $(let options = [$({$($args:tt)+}),* $(,)?];)?
    })) => {
        {
            let description = ::doop_localizer::localize!("subcommand.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::SubCommandBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("subcommand.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("subcommand.{}.{}.description", Self::NAME, $name))
                $($(.option($crate::command!(@option({$($args)+}))))*)?
        }
    };
    (@option({
        let name = $name:literal;
        let kind = SubCommandGroup;
        $(let commands = [$({$($args:tt)+}),* $(,)?];)?
    })) => {
        {
            let description = ::doop_localizer::localize!("subgroup.{}.{}.description", Self::NAME, $name);
            let mut builder = ::twilight_util::builder::command::SubCommandGroupBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("subgroup.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("subgroup.{}.{}.description", Self::NAME, $name));

            $(
                let commands = [$($crate::command!(@option({$($args)+}))),*];

                if !commands.is_empty() { builder = builder.subcommands(commands.to_vec()); }
            )?

            builder
        }
    };
    (@option({
        let name = $name:literal;
        let kind = User;
        let required = $required:literal;
    })) => {
        {
            let description = ::doop_localizer::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::UserBuilder::new($name, description)
                .name_localizations(::doop_localizer::localize_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations(::doop_localizer::localize_map!("option.{}.{}.description", Self::NAME, $name))
                .required($required)
        }
    };
}
