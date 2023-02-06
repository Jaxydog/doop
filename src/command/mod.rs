use serenity::all::{ResolvedOption, ResolvedValue};

use crate::prelude::*;

pub mod embed;
pub mod help;
pub mod ping;

#[macro_export]
macro_rules! define_command {
    ($name:literal {
        $(description: $desc:literal,)?
        $(permissions: $perm:ident,)?
        $(allow_dms: $dms:literal,)?
        $(options: [ $($option:expr,)+ ],)?
    }) => {
        pub const NAME: &str = $name;

        pub fn new() -> CreateCommand {
            CreateCommand::new($name)
                $(.default_member_permissions(Permissions::$perm))?
                $(.description($desc))?
                $(.dm_permission($dms))?
                $(.set_options(vec![$($option,)+]))?
        }
    };
}
#[macro_export]
macro_rules! define_option {
    ($name:literal ($type:ident) {
        description: $description:literal,
        $(required: $required:literal,)?
        $(autocomplete: $autocomplete:literal,)?
        $(options: [ $($option:expr,)+ ],)?
        $(choices(i32): { $($iname:literal: $ichoice:expr,)+ },)?
        $(choices(str): { $($sname:literal: $schoice:expr,)+ },)?
        $(choices(f64): { $($fname:literal: $fchoice:expr,)+ },)?
        $(range(i32): $imin:literal..=$imax:literal,)?
        $(range(str): $smin:literal..=$smax:literal,)?
        $(range(f64): $fmin:literal..=$fmax:literal,)?
        $(channel_types: [ $($channel:ident,)+ ],)?
    }) => {
        CreateCommandOption::new(CommandOptionType::$type, $name, $description)
            $(.required($required))?
            $(.set_autocomplete($autocomplete))?
            $($(.add_sub_option($option))+)?
            $($(.add_int_choice($iname, $ichoice))+)?
            $($(.add_string_choice($sname, $schoice))+)?
            $($(.add_number_choice($fname, $fchoice))+)?
            $(.min_int_value($imin).max_int_value($imax))?
            $(.min_length($smin).clone().max_length($smax).clone())?
            $(.min_number_value($fmin).max_number_value($fmax))?
            $(.channel_types(vec![$(ChannelType::$channel,)+]))?
    };
}

macro_rules! define_getter {
    ($name:ident($inner:path) -> $output:ty) => {
        #[allow(dead_code)]
        pub fn $name<'c>(options: &'c [ResolvedOption<'c>], name: &'c str) -> Result<$output> {
            let resolved = options.iter().find(|r| r.name == name).map_or_else(
                || $crate::err_wrap!("missing data for \"{name}\""),
                |resolved| Ok(&resolved.value),
            )?;

            match resolved {
                $inner(v) => Ok(*v),
                _ => $crate::err_wrap!("invalid data type for \"{name}\""),
            }
        }
    };
    ($name:ident($inner:path) -> ref $output:ty) => {
        #[allow(dead_code)]
        pub fn $name<'c>(options: &'c [ResolvedOption<'c>], name: &'c str) -> Result<&'c $output> {
            let resolved = options.iter().find(|r| r.name == name).map_or_else(
                || $crate::err_wrap!("missing data for \"{name}\""),
                |resolved| Ok(&resolved.value),
            )?;

            match resolved {
                $inner(v) => Ok(v),
                _ => $crate::err_wrap!("invalid data type for \"{name}\""),
            }
        }
    };
}

define_getter!(get_bool(ResolvedValue::Boolean) -> bool);
define_getter!(get_i64(ResolvedValue::Integer) -> i64);
define_getter!(get_f64(ResolvedValue::Number) -> f64);
define_getter!(get_partial_channel(ResolvedValue::Channel) -> ref PartialChannel);
define_getter!(get_role(ResolvedValue::Role) -> ref Role);
define_getter!(get_str(ResolvedValue::String) -> ref str);
define_getter!(get_subcommand(ResolvedValue::SubCommand) -> ref [ResolvedOption<'c>]);
define_getter!(get_subcommand_group(ResolvedValue::SubCommandGroup) -> ref [ResolvedOption<'c>]);

#[allow(dead_code)]
pub fn get_user<'c>(
    options: &'c [ResolvedOption<'c>],
    name: &'c str,
) -> Result<(&'c User, Option<&'c PartialMember>)> {
    let resolved = options.iter().find(|r| r.name == name).map_or_else(
        || err_wrap!("missing data for \"{name}\""),
        |resolved| Ok(&resolved.value),
    )?;

    match resolved {
        ResolvedValue::User(user, member) => Ok((user, *member)),
        _ => err_wrap!("invalid data type for \"{name}\""),
    }
}

#[allow(dead_code)]
pub fn get_input_text<'c>(options: &'c [ActionRow], name: &'c str) -> Result<&'c str> {
    for row in options {
        let Some(ActionRowComponent::InputText(input)) = row.components.first() else {
            continue;
        };

        if input.custom_id == name && !input.value.is_empty() {
            return Ok(&input.value);
        }
    }

    err_wrap!("missing data for \"{name}\"")
}
