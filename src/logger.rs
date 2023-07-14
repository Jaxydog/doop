use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

use crate::utility::Result;

/// The formatter used to create logger file names.
pub const NAME_FORMAT: &[FormatItem<'static>] = format_description!(
    version = 2,
    "[year repr:last_two][month padding:zero repr:numerical][day padding:zero]-[hour padding:zero repr:24][minute padding:zero][second padding:zero][subsecond digits:6]"
);
/// The formatter used to create log time strings.
pub const TIME_FORMAT: &[FormatItem<'static>] = format_description!(
    version = 2,
    "[day padding:zero]-[month padding:zero repr:numerical]-[year repr:last_two] [hour padding:zero repr:24]:[minute padding:zero]:[second padding:zero].[subsecond digits:6]"
);

crate::global! {{
    /// Returns the bot's logger value.
    [LOGGER] fn logger() -> Logger;
}}

/// Stores a logger structure's runtime flags.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// Whether the logger should print to the console.
    pub no_print: bool,
    /// Whether the logger should write to a file.
    pub no_write: bool,
}

impl Flags {
    /// Creates a new flag data structure.
    #[must_use]
    pub fn new() -> Self {
        let args = crate::utility::arguments();

        Self { no_print: args.no_print_logs, no_write: args.no_write_logs }
    }
}

/// Provides an interface for logging to the console and file system.
#[derive(Debug)]
pub struct Logger {
    /// The logger's runtime flags.
    pub flags: Flags,
    /// The logger's storage file name.
    file: PathBuf,
}

impl Logger {
    /// The directory that stores all log files
    const DIR: &str = "logs";

    /// Creates a new logger instance
    #[allow(clippy::unwrap_used, clippy::missing_panics_doc)]
    #[must_use]
    pub fn new() -> Self {
        let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let name = time.format(NAME_FORMAT).unwrap();
        let file = PathBuf::from(Self::DIR).join(name).with_extension("txt");

        Self { flags: Flags::new(), file }
    }

    /// Outputs a log to the enabled logger destinations
    pub fn output(&self, kind: &str, text: impl Display) -> Result<()> {
        if self.flags.no_print && self.flags.no_write {
            return Ok(());
        }

        let now = OffsetDateTime::now_local()?.format(TIME_FORMAT)?;
        let log = format!("[{now}] ({kind}) {text}");

        if !self.flags.no_print {
            let mut lock = std::io::stdout().lock();

            writeln!(lock, "{log}")?;
        }
        if !self.flags.no_write {
            self.file.parent().map_or(Ok(()), std::fs::create_dir_all)?;

            let mut file = File::options().append(true).create(true).open(&self.file)?;

            writeln!(file, "{log}")?;
        }

        Ok(())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

/// Outputs a debug log.
///
/// # Examples
/// ```
/// debug!("this is a debug log!")?;
/// ```
#[macro_export]
macro_rules! debug {
    ($($args:tt)+) => {
        if ::std::cfg!(debug_assertions) {
            $crate::logger::logger().output("DEBUG", &format!($($args)+))
        } else {
            Ok(())
        }
    };
}
/// Outputs an info log.
///
/// # Examples
/// ```
/// info!("this is an info log!")?;
/// ```
#[macro_export]
macro_rules! info {
    ($($args:tt)+) => {
        $crate::logger::logger().output("INFO", &format!($($args)+))
    };
}

/// Outputs a warning log.
///
/// # Examples
/// ```
/// warn!("this is a warning log!")?;
/// ```
#[macro_export]
macro_rules! warn {
    ($($args:tt)+) => {
        $crate::logger::logger().output("WARN", &format!($($args)+))
    };
}

/// Outputs an error log.
///
/// # Examples
/// ```
/// error!("this is an error log!")?;
/// ```
#[macro_export]
macro_rules! error {
    ($($args:tt)+) => {
        $crate::logger::logger().output("ERROR", &format!($($args)+))
    };
}
