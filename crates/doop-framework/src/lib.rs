#![doc = env!("CARGO_PKG_DESCRIPTION")]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]

use twilight_model::application::command::{CommandOptionType, CommandType};

struct FrameworkCommand {
    pub name: Box<str>,
    pub kind: CommandType,
    pub allow_dms: bool,
    pub is_nsfw: bool,
    pub options: Vec<FrameworkOption>,
}

struct FrameworkOption {
    pub name: Box<str>,
    pub kind: CommandOptionType,
    pub is_required: bool,
    pub data: FrameworkOptionData,
}

enum FrameworkOptionData {
    SubCommand {},
    SubCommandGroup {},
    String {
        autocomplete: Option<bool>,
        default: Option<String>,
        min: Option<usize>,
        max: Option<usize>,
        choices: Option<Vec<(Box<str>, String)>>,
    },
    Integer {
        autocomplete: Option<bool>,
        default: Option<i64>,
        min: Option<i64>,
        max: Option<i64>,
        choices: Option<Vec<(Box<str>, i64)>>,
    },
    Boolean {
        default: Option<bool>,
    },
    User {},
    Channel {},
    Role {},
    Mentionable {},
    Number {
        autocomplete: Option<bool>,
        default: Option<f64>,
        min: Option<f64>,
        max: Option<f64>,
        choices: Option<Vec<(Box<str>, f64)>>,
    },
    Attachment {},
}

/// A framework command implementation.
pub trait FrameworkCommand_ {
    /// Returns the name of this [`FrameworkCommand`].
    fn name(&self) -> &str;

    /// Returns the type of this [`CommandType`].
    fn kind(&self) -> CommandType;

    /// Returns whether this [`FrameworkCommand`] allows DM usage.
    fn allow_dms(&self) -> bool;

    /// Returns whether this [`FrameworkCommand`] is considered NSFW.
    fn is_nsfw(&self) -> bool;

    /// Returns the option names and types of this [`FrameworkCommand`].
    fn options(&self) -> &[&dyn FrameworkOption_];
}

/// A framework option implementation.
pub trait FrameworkOption_ {
    /// Returns the name of this [`FrameworkOption`].
    fn name(&self) -> &str;

    /// Returns the type of this [`FrameworkOption`].
    fn kind(&self) -> CommandOptionType;

    /// Returns whether this [`FrameworkOption`] is required.
    fn is_required(&self) -> bool;
}

// #[command(kind = ChatInput, dms = true, nsfw = false)]
// #[restrict(MANAGE_MESSAGES, USE_SLASH_COMMANDS)]
// fn embed(
//     ctx: (),
//     #[option(max = 256)] title: Option<&str>,
//     #[option(max = 4096)] description: Option<&str>,
//     #[option(min = -1, max = 0xFF_FF_FF, default = -1)] color: i64,
//     #[option(default = false)] ephemeral: bool,
// ) {
// }
