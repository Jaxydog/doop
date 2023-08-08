#![doc = env!("CARGO_PKG_DESCRIPTION")]
#![forbid(clippy::panic, clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]

use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

/// Formats log file names.
const NAME_FORMAT: &[FormatItem<'static>] = format_description!(
    version = 2,
    "[year repr:last_two][month padding:zero repr:numerical][day padding:zero]-[hour padding:zero repr:24][minute padding:zero][second padding:zero][subsecond digits:6]"
);
/// Formats log timestamps.
const TIME_FORMAT: &[FormatItem<'static>] = format_description!(
    version = 2,
    "[day padding:zero]-[month padding:zero repr:numerical]-[year repr:last_two] [hour padding:zero repr:24]:[minute padding:zero]:[second padding:zero].[subsecond digits:6]"
);

doop_macros::global! {
    /// The directory that stores the bot's logs.
    static LOG_DIRECTORY: Box<Path> = PathBuf::from("logs").into();
    /// The directory that stores the bot's errors.
    static ERROR_DIRECTORY: Box<Path> = PathBuf::from("logs/.error").into();
    /// The logger instance.
    static LOGGER: Logger = Logger::default();
}

/// Installs and configures the logger instance with the given output paths.
///
/// # Panics
///
/// Panics if the logger or either log directory has already been initialized.
#[allow(clippy::expect_used)]
pub fn install_into(flags: Flags, log_dir: impl AsRef<OsStr>, error_dir: impl AsRef<OsStr>) {
    LOG_DIRECTORY
        .set(PathBuf::from(log_dir.as_ref()).into())
        .expect("the log directory has already been initialized");
    ERROR_DIRECTORY
        .set(PathBuf::from(error_dir.as_ref()).into())
        .expect("the error directory has already been initialized");
    LOGGER
        .set(Logger::new(flags))
        .expect("the logger instance has already been initialized");
}

/// Installs and configures the logger instance.
///
/// # Panics
///
/// Panics if the logger or either log directory has already been initialized.
#[inline]
pub fn install(flags: Flags) { crate::install_into(flags, "logs", "logs/.error"); }

/// An [`Error`] that can occur during usage of the storage system.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error during IO.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error during formatting.
    #[error(transparent)]
    Format(#[from] time::error::Format),
}

/// Stores a logger's runtime flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// Disables console output.
    pub no_print: bool,
    /// Disables log file output.
    pub no_write: bool,
    /// Disables error log output.
    pub no_error: bool,
}

/// A logging interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logger {
    /// The logger's runtime flags.
    flags: Flags,
    /// The current log file path.
    file: Box<Path>,
}

impl Logger {
    /// Creates a new [`Logger`].
    ///
    /// # Panics
    ///
    /// Panics if the logger's log file path could not be created.
    #[must_use]
    pub fn new(flags: Flags) -> Self {
        #[allow(clippy::expect_used)]
        let name = Self::now().format(NAME_FORMAT).expect("unable to create log file path");
        let file = log_directory().join(name).with_extension("txt").into_boxed_path();

        Self { flags, file }
    }

    /// Returns the current [`OffsetDateTime`].
    #[inline]
    fn now() -> OffsetDateTime {
        OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
    }

    /// Creates a new error log file path.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file path could not be created.
    fn new_error_file() -> Result<Box<Path>, time::error::Format> {
        let name = Self::now().format(NAME_FORMAT)?;
        let file = error_directory().join(name).with_extension("txt");

        Ok(file.into_boxed_path())
    }

    /// Outputs a log.
    ///
    /// # Errors
    ///
    /// This function will return an error if the log could not be displayed or the log file(s)
    /// could not be created.
    pub fn output(&self, kind: &str, text: impl Display, is_error: bool) -> Result<(), Error> {
        if self.flags.no_print && self.flags.no_write {
            return Ok(());
        }

        let kind = kind.to_lowercase();
        let time = Self::now().format(TIME_FORMAT)?;
        let content = format!("[{time}] ({kind}) {text}");

        if !self.flags.no_print {
            let mut lock = std::io::stdout().lock();

            writeln!(lock, "{content}")?;
        }

        if !self.flags.no_write {
            self.file.parent().map_or(Ok(()), std::fs::create_dir_all)?;

            let mut file = File::options().append(true).create(true).open(&self.file)?;

            writeln!(file, "{content}")?;

            if is_error {
                let path = Self::new_error_file()?;

                path.parent().map_or(Ok(()), std::fs::create_dir_all)?;

                let mut file = File::options().append(true).create(true).open(&path)?;

                writeln!(file, "TYPE: {kind}\nTIME: {time}\nTEXT: {text}")?;
            }
        }

        Ok(())
    }
}

impl Default for Logger {
    #[inline]
    fn default() -> Self { Self::new(Flags::default()) }
}

/// Outputs a debug log.
///
/// # Examples
///
/// ```
/// debug!("this is a debug log!")?; 
/// ```
#[macro_export]
macro_rules! debug {
    ($($args:tt)+) => {
		if ::std::cfg!(debug_assertions) {
			$crate::logger().output("debug", format_args!($($args)+), false)
		} else {
			Ok::<(), $crate::Error>(())
		}
	};
}

/// Outputs an info log.
///
/// # Examples
///
/// ```
/// info!("this is an info log!")?; 
/// ```
#[macro_export]
macro_rules! info {
    ($($args:tt)+) => { $crate::logger().output("info", format_args!($($args)+), false) };
}

/// Outputs a warning log.
///
/// # Examples
///
/// ```
/// warn!("this is a warning log!")?; 
/// ```
#[macro_export]
macro_rules! warn {
    ($($args:tt)+) => { $crate::logger().output("warn", format_args!($($args)+), false) };
}

/// Outputs an error log.
///
/// # Examples
///
/// ```
/// error!("this is an error log!")?; 
/// ```
#[macro_export]
macro_rules! error {
    ($($args:tt)+) => { $crate::logger().output("error", format_args!($($args)+), true) };
}
