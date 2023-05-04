use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Local};
use owo_colors::OwoColorize;

use super::{get_arguments, Result};

/// Defines a log's kind
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    /// A regular info log
    Info,
    /// A warning log
    Warn,
    /// An error log
    Error,
}

/// Defines a single log entry
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Log<'lg> {
    /// The time that the log was created
    pub time: DateTime<Local>,
    /// The type of log
    pub kind: LogKind,
    /// The log's text
    pub text: &'lg str,
}

impl<'lg> Log<'lg> {
    /// Creates a new log entry
    #[must_use]
    pub fn new(kind: LogKind, text: &'lg str) -> Self {
        let time = Local::now();

        Self { time, kind, text }
    }

    /// Returns the log's time string
    #[must_use]
    pub fn get_time(&self, with_color: bool) -> String {
        // Saving these `l` and `r` variables just feels... wrong...
        // *But* it's helpful for when we have color to be able to have these values in
        // one spot like this.
        let time = self.time.format("%x %X%.3f");
        let (l, r) = ('[', ']');

        if with_color {
            format!("{}{}{}", l.bright_black(), time.dimmed(), r.bright_black())
        } else {
            format!("{l}{time}{r}")
        }
    }

    /// Returns the log's kind string
    #[must_use]
    pub fn get_kind(&self, with_color: bool) -> String {
        let (l, r) = ('(', ')');

        // Early return saving me from a fat if / else statement.
        if !with_color {
            return format!("{l}{:?}{r}", self.kind).to_lowercase();
        }

        let (l, r) = (l.bright_black(), r.bright_black());
        let kind = format!("{:?}", self.kind).to_lowercase();

        match self.kind {
            LogKind::Info => format!("{l}{}{r}", kind.bright_blue()),
            LogKind::Warn => format!("{l}{}{r}", kind.yellow()),
            LogKind::Error => format!("{l}{}{r}", kind.bright_red()),
        }
    }

    /// Returns the log's text string
    #[must_use]
    pub fn get_text(&self, with_color: bool) -> String {
        if with_color {
            self.text.bright_white().to_string()
        } else {
            self.text.to_string()
        }
    }

    /// Returns the log's entry string
    #[must_use]
    pub fn get_entry(&self, with_color: bool) -> String {
        let time = self.get_time(with_color);
        let kind = self.get_kind(with_color);
        let text = self.get_text(with_color);

        format!("{time} {kind} {text}")
    }
}

/// Provides an interface for log output and file storage
#[derive(Debug)]
pub struct Logger {
    /// Whether the logger should output to the console
    pub quiet: bool,
    /// The logger's log file path, or [`None`] if storage is disabled
    path: Option<PathBuf>,
}

impl Logger {
    /// The directory to store log files within
    pub const LOG_DIR: &str = "logs";
    /// The file extension of log files
    pub const LOG_EXT: &str = "txt";

    /// Creates a new logger instance
    #[must_use]
    pub fn new() -> Self {
        let args = get_arguments();
        let quiet = args.disable_log_output;
        let path = (!args.disable_log_storage).then(|| {
            let file = Local::now().format("%y%m%d_%H%M%S_%6f").to_string();
            let dir = PathBuf::from(Self::LOG_DIR);

            dir.join(file).with_extension(Self::LOG_EXT)
        });

        Self { quiet, path }
    }

    /// Outputs the given log
    pub fn output(&self, kind: LogKind, text: &'_ str) -> Result<()> {
        let log = Log::new(kind, text);

        if !self.quiet {
            println!("{}", log.get_entry(true));
        }
        if let Some(path) = self.path.as_deref() {
            // Having this call every time sucks, but it's better to be sure that the
            // directory is actually made so we don't throw an error every time we try to
            // output a log.
            //
            // Since the directory will exist after one call, or at least it should, I
            // imagine the overhead will be minimal.
            create_dir_all(Self::LOG_DIR)?;

            let mut file = File::options().append(true).create(true).open(path)?;

            file.write_all(log.get_entry(false).as_bytes())?;
        }

        Ok(())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
