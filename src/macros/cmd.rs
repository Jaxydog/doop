/// Defines and implements a command state.
///
/// # Examples
/// ```
/// command! {
///     TYPE = ChatInput,
///     NAME = "ping",
///     REQUIRES = [USE_SLASH_COMMANDS],
/// }
/// ```
#[macro_export]
macro_rules! command {
    {
        TYPE = $type:ident,
        NAME = $name:literal,
        $( DMS = $dms:literal, )?
        $( NSFW = $nsfw:literal, )?
        $( REQUIRES = [ $( $permission:ident ),* $(,)? ], )?
        $( OPTIONS = [ $( $option:expr ),* $(,)? ], )?
    } => {
        /// The command's state.
        #[derive(::std::fmt::Debug, ::std::default::Default)]
        pub struct This;

        impl This {
            #[allow(dead_code)]
            const NAME: &'static str = $name;
        }

        impl $crate::command::DoopCommand for This {
            fn name(&self) -> &'static str {
                $name
            }

            fn kind(&self) -> ::twilight_model::application::command::CommandType {
                ::twilight_model::application::command::CommandType::$type
            }

            fn build(
                &self,
                guild_id: ::std::option::Option<::twilight_model::id::Id<::twilight_model::id::marker::GuildMarker>>
            ) -> ::anyhow::Result<::twilight_model::application::command::Command> {
                let description = $crate::localize!("command.{}.description", self.name());
                let mut builder = ::twilight_util::builder::command::CommandBuilder::new(self.name(), description, self.kind())
                    .name_localizations($crate::locale_map!("command.{}.name", self.name()))
                    .description_localizations($crate::locale_map!("command.{}.description", self.name()))
                    $(.dm_permission($dms))?
                    $(.nsfw($nsfw))?
                    $($(.option($option))*)?;

                if let Some(guild_id) = guild_id {
                    builder = builder.guild_id(guild_id);
                }

                $(
                    let permissions = ::twilight_model::guild::Permissions::empty()
                        $(.union(::twilight_model::guild::Permissions::$permission))*;

                    builder = builder.default_member_permissions(permissions);
                )?

                Ok(builder.validate()?.build())
            }
        }
    };
}

/// Defines and builds a command option.
///
/// # Examples
/// ```
/// option! {
///     TYPE = Boolean,
///     NAME = "ephemeral",
///     REQUIRED = true,
/// }
/// ```
#[macro_export]
macro_rules! option {
    {
        TYPE = Attachment,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::AttachmentBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
        }
    };
    {
        TYPE = Boolean,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::BooleanBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
        }
    };
    {
        TYPE = Channel,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
        $( KINDS = [ $( $kind:ident ),* $(,)? ], )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::ChannelBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
                $(.channel_types([$( ::twilight_model::channel::ChannelType::$kind ),*]))?
        }
    };
    {
        TYPE = Integer,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
        $( AUTOFILL = $autofill:literal, )?
        $( CHOICES = [ $( ($choice:literal, $value:expr) ),* $(,)? ], )?
        $( MIN = $min:expr, )?
        $( MAX = $max:expr, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);
            let mut builder = ::twilight_util::builder::command::IntegerBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
                $(.autocomplete($autofill))?
                $(.min_value($min))?
                $(.max_value($max))?;

            let choices: &[(&str, _)] = &[$($( ($choice, $value) ),*)?];

            if !choices.is_empty() {
                builder = builder.choices(choices.to_vec());

                for (choice, _) in choices {
                    let localized = $crate::locale_map!("choice.{}.{}.{choice}", Self::NAME, $name);

                    builder = builder.choice_localizations(choice, localized);
                }
            }

            builder
        }
    };
    {
        TYPE = Mentionable,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::MentionableBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
        }
    };
    {
        TYPE = Number,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
        $( AUTOFILL = $autofill:literal, )?
        $( CHOICES = [ $( ($choice:literal, $value:expr) ),* $(,)? ], )?
        $( MIN = $min:expr, )?
        $( MAX = $max:expr, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);
            let mut builder = ::twilight_util::builder::command::NumberBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
                $(.autocomplete($autofill))?
                $(.min_value($min))?
                $(.max_value($max))?;

            let choices: &[(&str, _)] = &[$($( ($choice, $value) ),*)?];

            if !choices.is_empty() {
                builder = builder.choices(choices.to_vec());

                for (choice, _) in choices {
                    let localized = $crate::locale_map!("choice.{}.{}.{choice}", Self::NAME, $name);

                    builder = builder.choice_localizations(choice, localized);
                }
            }

            builder
        }
    };
    {
        TYPE = Role,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::RoleBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
        }
    };
    {
        TYPE = String,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
        $( AUTOFILL = $autofill:literal, )?
        $( CHOICES = [ $( ($choice:literal, $value:expr) ),* $(,)? ], )?
        $( MIN = $min:expr, )?
        $( MAX = $max:expr, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);
            #[allow(unused_mut)]
            let mut builder = ::twilight_util::builder::command::StringBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
                $(.autocomplete($autofill))?
                $(.min_length($min))?
                $(.max_length($max))?;

            $(
                let choices: &[(&str, _)] = &[$( ($choice, $value) ),*];

                if !choices.is_empty() {
                    builder = builder.choices(choices.to_vec());

                    for (choice, _) in choices {
                        let localized = $crate::locale_map!("choice.{}.{}.{choice}", Self::NAME, $name);

                        builder = builder.choice_localizations(choice, localized);
                    }
                }
            )?

            builder
        }
    };
    {
        TYPE = SubCommand,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
        $( OPTIONS = [ $( $option:expr ),* $(,)? ], )?
    } => {
        {
            let description = $crate::localize!("subcommand.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::SubCommandBuilder::new($name, description)
                .name_localizations($crate::locale_map!("subcommand.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("subcommand.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
                $($(.option($option))*)?
        }
    };
    {
        TYPE = SubCommandGroup,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
        $( COMMANDS = [ $( $command:expr ),* $(,)? ], )?
    } => {
        {
            let description = $crate::localize!("group.{}.{}.description", Self::NAME, $name);
            let mut builder = ::twilight_util::builder::command::SubCommandGroupBuilder::new($name, description)
                .name_localizations($crate::locale_map!("group.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("group.{}.{}.description", Self::NAME, $name))
                $(.required($required))?;

            let commands: &[::twilight_util::builder::command::SubCommandBuilder] = &[$($($command),*)?];

            if !commands.is_empty() {
                builder = builder.subcommands(commands.to_vec());
            }

            builder
        }
    };
    {
        TYPE = User,
        NAME = $name:literal,
        $( REQUIRED = $required:literal, )?
    } => {
        {
            let description = $crate::localize!("option.{}.{}.description", Self::NAME, $name);

            ::twilight_util::builder::command::UserBuilder::new($name, description)
                .name_localizations($crate::locale_map!("option.{}.{}.name", Self::NAME, $name))
                .description_localizations($crate::locale_map!("option.{}.{}.description", Self::NAME, $name))
                $(.required($required))?
        }
    };
}
