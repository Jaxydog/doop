use std::path::PathBuf;

use clap::Parser;
use doop_localizer::Locale;
use twilight_model::channel::message::component::{Button, TextInput};
use twilight_model::channel::message::Component;

use self::builder::ActionRowBuilder;

/// Provides model builders.
pub mod builder;
/// Provides getters for various client secrets.
pub mod env;
/// Provides type extension traits.
pub mod ext;

doop_macros::global! {
    /// The bot's command-line arguments.
    static ARGUMENTS: Arguments = Arguments::parse();
}

/// The bot's branding color.
pub const BRANDING: u32 = 0x24_9F_DE;
/// The bot's success color.
pub const SUCCESS: u32 = 0x59_C1_35;
/// The bot's failure color.
pub const FAILURE: u32 = 0xB4_20_2A;

/// Wraps an [`anyhow::Result<T, E>`], providing a defaulted `T` generic type.
pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

/// The bot's command-line arguments.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq, Parser)]
#[command(author, about, version)]
pub struct Arguments {
    /// Disables logger printing.
    #[arg(short = 'q', long = "quiet")]
    pub log_no_print: bool,
    /// Disables log file writing.
    #[arg(short = 'e', long = "ephemeral")]
    pub log_no_write: bool,
    /// Disables error log file writing.
    #[arg(short = 'E', long = "ephemeral-errors")]
    pub log_no_error: bool,

    /// The bot's preferred locale.
    #[arg(short = 'l', long = "preferred-locale")]
    pub lang_prefer_locale: Option<Locale>,

    /// The directory to store log files within.
    #[arg(long = "log-directory")]
    pub log_write_dir: Option<PathBuf>,
    /// The directory to store error log files within.
    #[arg(long = "error-log-directory")]
    pub log_error_dir: Option<PathBuf>,
    /// The directory that contains the bot's localization files.
    #[arg(short = 'L', long = "lang-directory")]
    pub lang_file_dir: Option<PathBuf>,
    /// The directory that contains the bot's data files.
    #[arg(short = 'd', long = "data-directory")]
    pub data_file_dir: Option<PathBuf>,
}

/// Automatically sorts buttons into action rows.
pub fn button_rows(buttons: impl IntoIterator<Item = impl Into<Button>>) -> Vec<Component> {
    let mut components = Vec::with_capacity(5);
    let mut action_row = Vec::with_capacity(5);

    for button in buttons {
        if action_row.len() < 5 {
            action_row.push(Component::Button(button.into()));
        } else {
            components.push(ActionRowBuilder::new(action_row).into());
            action_row = Vec::with_capacity(5);
        }
    }

    if !action_row.is_empty() {
        components.push(ActionRowBuilder::new(action_row).into());
    }

    components
}

/// Automatically sorts text inputs into action rows.
#[inline]
pub fn text_input_rows(inputs: impl IntoIterator<Item = impl Into<TextInput>>) -> Vec<Component> {
    inputs
        .into_iter()
        .map(Into::into)
        .map(|i| Component::from(ActionRowBuilder::new([i])))
        .collect()
}
