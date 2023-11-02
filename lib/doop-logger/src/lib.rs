//! Implements a terminal and file system logger for the Doop Discord bot.
#![deny(clippy::expect_used, unsafe_code, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::todo, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use crossbeam_channel::{SendError, Sender};
use doop_threads::{AutoJoin, Consumer, HandledThread, SenderThread};
use owo_colors::{OwoColorize, Stream};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

/// The type of the returned logging thread handle.
pub type LogThread = AutoJoin<Consumer<Message, std::io::Result<()>>, std::io::Result<()>>;

/// The logging thread's sender channel.
static SENDER: OnceLock<Sender<Message>> = OnceLock::new();

/// Returns a reference to the logging thread's sender channel.
///
/// # Panics
///
/// Panics if the logging thread has not been initialized.
#[allow(clippy::expect_used)]
fn sender() -> &'static Sender<Message> {
    SENDER.get().expect("the logging thread has not been initialized")
}

/// Queues a log.
///
/// # Errors
///
/// This function will return an error if the logging thread is closed.
pub fn queue(kind: Level, text: impl Display) -> Result<(), SendError<Message>> {
    let log = Log::new(Time::now(), kind, text);

    sender().send(Message::Queue(log))
}

/// Flushes the logger queue.
///
/// # Errors
///
/// This function will return an error if the logging thread is closed.
pub fn flush() -> Result<(), SendError<Message>> {
    sender().send(Message::Flush)
}

/// Closes the logging thread.
///
/// If you call this method, the thread cannot be safely re-initialized and attempting to log again
/// will always return an error.
///
/// # Errors
///
/// This function will return an error if the logging thread is closed.
pub fn close() -> Result<(), SendError<Message>> {
    sender().send(Message::Close)
}

/// Initializes the logging thread.
///
/// The returned thread handle cannot be dropped, or else the logging thread will close.
///
/// # Panics
///
/// Panics if the thread cannot be closed when the handle is dropped, or if a thread has already
/// been initialized.
///
/// # Errors
///
/// This function will return an error if the thread cannot be initialized.
pub fn install(config: Config, dir: impl AsRef<Path>) -> std::io::Result<LogThread> {
    let mut logger = Logger::new(config, dir);
    let timeout = Duration::from_millis(logger.config.stale_time);

    let thread = Consumer::spawn("logger", move |receiver| {
        use crossbeam_channel::RecvTimeoutError::{Disconnected, Timeout};

        loop {
            match receiver.recv_timeout(timeout) {
                Ok(Message::Queue(log)) if !logger.config.disabled() => logger.queue(log)?,
                Ok(Message::Flush) | Err(Timeout) if !logger.is_empty() => logger.flush()?,
                Ok(Message::Close) | Err(Disconnected) => {
                    drop(receiver);

                    return logger.flush();
                }
                _ => {}
            }
        }
    })?;

    // Multiple threads should not be initialized; doing so could cause output inconsistencies.
    #[allow(clippy::expect_used)]
    SENDER.set(thread.clone_sender()).expect("the logging thread has already been initialized");

    // If the call to `close` fails the logging thread may never join.
    #[allow(clippy::unwrap_used)]
    Ok(thread.auto_cleaned(|_| close().unwrap()))
}

/// A message to be sent to the logging thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// Outputs a log.
    Queue(Log),
    /// Flushes the logger.
    Flush,
    /// Closes the logging thread.
    Close,
}

/// A logger instance.
#[derive(Debug)]
pub struct Logger {
    /// The logger's configuration.
    config: Config,
    /// The logger's output file path.
    path: Box<Path>,
    /// The logger's output queue.
    queue: Vec<Log>,
}

impl Logger {
    /// A time format for log file names.
    pub const FILENAME_FORMAT: &'static [FormatItem<'static>] = format_description!(
        version = 2,
        "[year repr:last_two][month padding:zero repr:numerical][day padding:zero]-[hour \
         padding:zero repr:24][minute padding:zero][second padding:zero][subsecond digits:6]"
    );

    /// Creates a new [`Logger`].
    ///
    /// # Panics
    ///
    /// Panics if the defined file name formatter is invalid.
    #[must_use]
    pub fn new(config: Config, dir: impl AsRef<Path>) -> Self {
        let time = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        #[allow(clippy::unwrap_used)] // Will only fail if the format constant is invalid.
        let file = time.format(Self::FILENAME_FORMAT).unwrap();
        let path = dir.as_ref().join(file).with_extension("txt").into_boxed_path();

        Self { config, path, queue: Vec::with_capacity(config.queue_size) }
    }

    /// Returns whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns whether the queue is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.config.queue_size
    }

    /// Appends a log to the queue, flushing the logger if its capacity is met or exceeded.
    ///
    /// # Errors
    ///
    /// This function will return an error if log(s) failed to output during a flush.
    pub fn queue(&mut self, log: Log) -> std::io::Result<()> {
        self.queue.push(log);

        if self.is_full() { self.flush() } else { Ok(()) }
    }

    /// Flushes the output queue of this [`Logger`].
    ///
    /// # Errors
    ///
    /// This function will return an error if log(s) failed to output.
    pub fn flush(&mut self) -> std::io::Result<()> {
        let display_color = self.config.support_color();
        let mut iterator = self.queue.drain(..).fuse().peekable();
        let mut blocks = vec![];

        while let Some(log) = iterator.next() {
            let color = display_color.then(|| log.display(Some(log.stream())));
            let mut block = (log.kind.error, log.display(None), color);

            #[allow(unsafe_code)]
            while iterator.peek().is_some_and(|l| l.kind.error == block.0) {
                // Safety: the loop's check guarantees that `.next()` always returns `Some`.
                let log = unsafe { iterator.next().unwrap_unchecked() };

                block.1.push('\n');
                block.1.push_str(&log.display(None));

                if let Some(ref mut string) = block.2 {
                    string.push('\n');
                    string.push_str(&log.display(Some(log.stream())));
                }
            }

            blocks.push(block);
        }

        if self.config.print {
            let mut out = None;
            let mut err = None;

            for (error, display) in blocks.iter().map(|(e, d, c)| (e, c.as_ref().unwrap_or(d))) {
                if *error {
                    writeln!(out.get_or_insert_with(|| std::io::stdout().lock()), "{display}")?;
                } else {
                    writeln!(err.get_or_insert_with(|| std::io::stderr().lock()), "{display}")?;
                }
            }
        }

        if self.config.write {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir)?;
            }

            let mut file = File::options().append(true).create(true).open(&self.path)?;
            let buffer = blocks.into_iter().map(|(_, d, _)| d).collect::<Box<_>>().join("\n");

            writeln!(file, "{buffer}")?;
        }

        Ok(())
    }
}

/// A logger configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Whether console output is disabled.
    pub print: bool,
    /// Whether file writing is disabled.
    pub write: bool,
    /// Whether console colors are disabled.
    pub color: bool,
    /// The logger's output queue capacity.
    pub queue_size: usize,
    /// The logger's output queue timeout in milliseconds.
    pub stale_time: u64,
}

impl Config {
    /// Returns whether this [`Config`] has logging disabled entirely.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        !(self.print || self.write) || self.queue_size == 0
    }

    /// Returns whether this [`Config`] allows color support.
    #[must_use]
    pub const fn support_color(&self) -> bool {
        self.print && self.color
    }
}

/// A log entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    /// The log's timestamp.
    pub time: Time,
    /// The log's level.
    pub kind: Level,
    /// The log's text.
    pub text: Box<str>,
}

impl Log {
    /// Creates a new [`Log`].
    pub fn new(time: Time, kind: Level, text: impl Display) -> Self {
        Self { time, kind, text: text.to_string().into_boxed_str() }
    }

    /// Returns the preferred output stream of this [`Log`].
    #[must_use]
    pub const fn stream(&self) -> Stream {
        if self.kind.error { Stream::Stderr } else { Stream::Stdout }
    }

    /// Formats and returns a display string representing this log.
    #[must_use]
    pub fn display(&self, color_stream: Option<Stream>) -> String {
        let time = self.time.display(color_stream);
        let kind = self.kind.display(color_stream);

        format!("{time} {kind} {}", self.text)
    }
}

/// A log timestamp.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time {
    inner: OffsetDateTime,
}

impl Time {
    /// A time format for log headers.
    pub const FORMAT: &'static [FormatItem<'static>] = format_description!(
        version = 2,
        "\\[[day padding:zero]-[month padding:zero repr:numerical]-[year repr:last_two] [hour \
         padding:zero repr:24]:[minute padding:zero]:[second padding:zero].[subsecond digits:6]\\]"
    );

    /// Creates a new [`Time`].
    #[must_use]
    pub const fn new(inner: OffsetDateTime) -> Self {
        Self { inner }
    }

    /// Creates a new [`Time`] containing the current time.
    #[must_use]
    pub fn now() -> Self {
        Self::new(OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()))
    }

    /// Formats and returns a display string representing this timestamp.
    ///
    /// # Panics
    ///
    /// Panics if the defined log format is invalid.
    #[must_use]
    pub fn display(&self, color_stream: Option<Stream>) -> String {
        #[allow(clippy::unwrap_used)] // Will only fail if the format constant is invalid.
        let text = self.inner.format(Self::FORMAT).unwrap();

        if let Some(stream) = color_stream {
            text.if_supports_color(stream, |s| s.dimmed()).to_string()
        } else {
            text
        }
    }
}

/// A log level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    /// The log level's name.
    pub name: &'static str,
    /// Whether the log level is considered an error.
    pub error: bool,
    /// Colorizes a string with the associated level color.
    pub color: fn(&String) -> String,
}

impl Level {
    /// Creates a new [`Level`].
    pub const fn new(name: &'static str, error: bool, color: fn(&String) -> String) -> Self {
        Self { name, error, color }
    }

    /// Formats and returns a display string representing this log level.
    #[must_use]
    pub fn display(&self, color_stream: Option<Stream>) -> String {
        let text = format!("({})", self.name);

        if let Some(stream) = color_stream {
            text.if_supports_color(stream, self.color).to_string()
        } else {
            text
        }
    }
}

/// Defines log levels.
macro_rules! levels {
    {$($const:ident($name:literal, $error:literal, $color:ident),)* $(,)?} => {
        impl Level {$(
            #[doc = concat!("The ", $name, " logging level.")]
            pub const $const: Self = Self::new($name, $error, |s| ::owo_colors::OwoColorize::$color(s).to_string());
        )*}
    };
}

levels! {
    DEBUG("debug", false, bright_purple),
    INFO("info", false, bright_blue),
    WARN("warn", false, bright_yellow),
    ERROR("error", false, bright_red),
}

/// Outputs a debug log.
///
/// ```
/// debug!("This is an debug log!")?;
/// ```
#[macro_export]
macro_rules! debug {
    ($($args:tt)+) => {
        if ::std::cfg!(debug_assertions) {
            $crate::queue($crate::Level::DEBUG, ::std::format_args!($($args)+))
        } else {
            ::std::result::Result::<(), ::std::sync::mpsc::SendError<$crate::Message>>::Ok(())
        }
    };
}

/// Outputs an info log.
///
/// ```
/// info!("This is an info log!")?;
/// ```
#[macro_export]
macro_rules! info {
    ($($args:tt)+) => {
        $crate::queue($crate::Level::INFO, ::std::format_args!($($args)+))
    };
}

/// Outputs a warn log.
///
/// ```
/// warn!("This is a warning log!")?;
/// ```
#[macro_export]
macro_rules! warn {
    ($($args:tt)+) => {
        $crate::queue($crate::Level::WARN, ::std::format_args!($($args)+))
    };
}

/// Outputs an error log.
///
/// ```
/// error!("This is an error log!")?;
/// ```
#[macro_export]
macro_rules! error {
    ($($args:tt)+) => {
        $crate::queue($crate::Level::ERROR, ::std::format_args!($($args)+))
    };
}
