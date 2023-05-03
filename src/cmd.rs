use std::sync::OnceLock;

use serenity::all::{
    ActionRow, ActionRowComponent, CommandInteraction, ModalInteraction, PartialChannel,
    PartialMember, ResolvedOption, ResolvedValue, Role, User,
};

use crate::err_wrap;
use crate::util::Result;

/// The embed command
pub mod embed;
/// The help command
pub mod help;
/// The mail command
pub mod mail;
/// The ping command
pub mod ping;
/// The role command
pub mod role;

/// Utility wrapper struct for resolving command data
#[derive(Clone, Debug)]
pub struct CommandDataResolver<'cmd> {
    command: &'cmd CommandInteraction,
    // I'm really happy that this was stablized. Being able to cache values after I load them for
    // the first time like this *and* having it be Send + Sync is *SO* nice.
    options: OnceLock<Vec<ResolvedOption<'cmd>>>,
}

impl<'cmd> CommandDataResolver<'cmd> {
    /// Creates a new command data resolver
    #[must_use]
    pub const fn new(command: &'cmd CommandInteraction) -> Self {
        let options = OnceLock::new();

        Self { command, options }
    }

    /// Creates a new command data resolver with the given options
    #[must_use]
    pub fn new_initialized(
        command: &'cmd CommandInteraction,
        resolved: Vec<ResolvedOption<'cmd>>,
    ) -> Self {
        let options = OnceLock::from(resolved);

        Self { command, options }
    }

    /// Returns the inner command
    pub const fn command(&self) -> &CommandInteraction {
        self.command
    }

    /// Returns the inner command's resolved options
    pub fn options<'opt: 'cmd>(&'cmd self) -> &'opt [ResolvedOption<'cmd>] {
        self.options.get_or_init(|| self.command().data.options())
    }

    // I'll... probably turn this into a macro in the future.
    // A *lot* of repeated code coming up.

    /// Returns a boolean from the command's options
    pub fn get_bool(&'cmd self, name: &str) -> Result<bool> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::Boolean(v) => Ok(*v),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns an integer from the command's options
    pub fn get_i64(&'cmd self, name: &str) -> Result<i64> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::Integer(v) => Ok(*v),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns a number from the command's options
    pub fn get_f64(&'cmd self, name: &str) -> Result<f64> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::Number(v) => Ok(*v),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns a partial channel from the command's options
    pub fn get_partial_channel(&'cmd self, name: &str) -> Result<&'cmd PartialChannel> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::Channel(v) => Ok(v),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns a role from the command's options
    pub fn get_role(&'cmd self, name: &str) -> Result<&'cmd Role> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::Role(v) => Ok(v),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns a string from the command's options
    pub fn get_str(&'cmd self, name: &str) -> Result<&'cmd str> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::String(v) => Ok(v),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns a user from the command's options
    pub fn get_user(&'cmd self, name: &str) -> Result<(&'cmd User, Option<&'cmd PartialMember>)> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        match resolved {
            ResolvedValue::User(u, m) => Ok((u, *m)),
            _ => err_wrap!("invalid data type for option '{name}'"),
        }
    }

    /// Returns a subcommand from the command's options
    pub fn get_subcommand(&'cmd self, name: &str) -> Result<Self> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        if let ResolvedValue::SubCommand(v) = resolved.clone() {
            Ok(CommandDataResolver::new_initialized(self.command(), v))
        } else {
            err_wrap!("invalid data type for option '{name}'")
        }
    }

    /// Returns a subcommand group from the command's options
    pub fn get_subcommand_group(&'cmd self, name: &str) -> Result<Self> {
        let resolved = self.options().iter().find(|r| r.name == name).map_or_else(
            || err_wrap!("missing data for option '{name}'"),
            |r| Ok(&r.value),
        )?;

        if let ResolvedValue::SubCommandGroup(v) = resolved.clone() {
            Ok(CommandDataResolver::new_initialized(self.command(), v))
        } else {
            err_wrap!("invalid data type for option '{name}'")
        }
    }
}

impl<'cmd> From<&'cmd CommandInteraction> for CommandDataResolver<'cmd> {
    fn from(value: &'cmd CommandInteraction) -> Self {
        Self::new(value)
    }
}

// Technically a wrapper for modals isn't needed in the same way that one for
// command data is needed, *but* it lets me future-proof this so I can expand it
// as modals are updated.

/// Utility wrapper struct for resolving modal data
#[derive(Clone, Debug)]
pub struct ModalDataResolver<'mdl> {
    modal: &'mdl ModalInteraction,
}

impl<'mdl> ModalDataResolver<'mdl> {
    /// Creates a new modal data resolver
    #[must_use]
    pub const fn new(modal: &'mdl ModalInteraction) -> Self {
        Self { modal }
    }

    /// Returns the inner modal
    #[must_use]
    pub const fn modal(&self) -> &ModalInteraction {
        self.modal
    }

    /// Returns the inner modal's rows
    #[must_use]
    pub fn rows(&self) -> &[ActionRow] {
        &self.modal.data.components
    }

    /// Returns a string from the modal's inputs
    pub fn get_input_text(&self, name: &str) -> Result<&str> {
        for row in self.rows() {
            // Having input components inside of their own personal action rows always
            // seemed really odd to me, but that's just how Discord wants it I guess...
            let Some(ActionRowComponent::InputText(input)) = row.components.first() else {
                continue;
            };

            if input.custom_id != name {
                continue;
            }
            if let Some(value) = input.value.as_deref() {
                return Ok(value);
            }
        }

        err_wrap!("missing data for input '{name}'")
    }
}

// And enter! The most helpful macro ever. I'm so tired of writing
// CommandData::new().whatever so many times.

/// Defines a new application command
#[macro_export]
macro_rules! command {
    ($name: literal: {
        $( description: $desc: literal, )?
        $( permissions: $perms: ident, )?
        $( dms_allowed: $dms: literal, )?
        $( options: [ $( $option: expr, )* ], )?
    }) => {
        /// The command's name
        pub const NAME: &str = $name;

        /// Returns the created command
        pub fn create() -> serenity::all::CreateCommand {
            serenity::all::CreateCommand::new(NAME)
                $( .description($desc) )?
                $( .default_member_permissions(serenity::all::Permissions::$perms) )?
                $( .dm_permission($dms) )?
                $( .set_options(vec![ $( $option ),* ]) )?
        }
    };
}

/// Defines a new application command option
#[macro_export]
macro_rules! option {
    ($name: literal <$type: ident>: {
        description: $desc: literal,
        $( required: $req: literal, )?
        $( autocomplete: $auto: literal, )?
        $( channels: [ $( $channel: ident ),* ], )?
        $( options: [ $( $option: expr, )* ], )?

        $( where <i32>: $imin: literal ..= $imax: literal, )?
        $( match <i32> { $( $ikey: expr => $ival: expr, )* }, )?

        $( where <f64>: $fmin: literal ..= $fmax: literal, )?
        $( match <f64> { $( $fkey: expr => $fval: expr, )* }, )?

        $( where <str>: $smin: literal ..= $smax: literal, )?
        $( match <str> { $( $skey: expr => $sval: expr, )* }, )?
    }) => {
        serenity::all::CreateCommandOption::new(serenity::all::CommandOptionType::$type, $name, $desc)
            $( .required($req) )?
            $( .set_autocomplete($auto) )?
            $( .channel_types(vec![ $( serenity::all::ChannelType::$channel, )* ]) )?
            $( $( .add_sub_option($option) )* )?
            $( .min_int_value($imin).max_int_value($imax) )?
            $( $( .add_int_choice($ikey, $ival) )* )?
            $( .min_number_value($fmin).max_number_value($fmax) )?
            $( $( .add_number_choice($fkey, $fval) )* )?
            $( .min_length($smin).max_length($smax).clone() )?
            $( $( .add_string_choice($skey, $sval) )* )?
    };
}
// I will never understand why `.min_length` and `.max_length` return a
// reference. I'll probably make a PR about this since it's really unfortunate
// having to clone like this.
