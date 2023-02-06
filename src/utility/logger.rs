use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use colored::Colorize;

use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogContent<'l> {
    pub time: DateTime<Local>,
    pub text: &'l str,
    pub error: bool,
}

impl<'l> LogContent<'l> {
    pub const LOG_KIND: &str = "[LOG]";
    pub const ERR_KIND: &str = "[ERR]";

    pub fn new(text: &'l str, error: bool) -> Self {
        Self {
            time: Local::now(),
            text,
            error,
        }
    }

    pub fn content_colored(&self) -> String {
        let time = self.time.format("[%x %X:%3f]").to_string().bright_black();
        let text = self.text.trim().bright_white();
        let kind = if self.error {
            Self::ERR_KIND.bright_red()
        } else {
            Self::LOG_KIND.bright_blue()
        };

        format!("{time} {kind} {text}")
    }
    pub fn content(&self) -> String {
        let time = self.time.format("[%x %X:%3f]").to_string();
        let text = self.text.trim();
        let kind = if self.error {
            Self::ERR_KIND
        } else {
            Self::LOG_KIND
        };

        format!("{time} {kind} {text}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logger {
    pub path: PathBuf,
    pub quiet: bool,
    pub store: bool,
}

impl Logger {
    pub const DIR: &str = "logs";

    pub fn new(quiet: bool, store: bool) -> Result<Self> {
        let name = Local::now().format("%y%m%d%H%M%S%f.txt").to_string();
        let path = PathBuf::from(Self::DIR).join(name);

        if store {
            create_dir_all(Self::DIR)?;

            File::create(&path)?;
        }

        Ok(Self { path, quiet, store })
    }

    fn log(&self, log: LogContent<'_>) -> Result<()> {
        if !self.quiet {
            println!("{}", log.content_colored());
        }
        if self.store {
            let mut file = File::options().append(true).open(&self.path)?;

            file.write_all(log.content().as_bytes())?;
            file.write_all(&[b'\n'])?;
            file.flush()?;
        }

        Ok(())
    }

    pub fn info<T>(&self, text: T) -> Result<()>
    where
        T: TryInto<String>,
        Error: From<<T as TryInto<String>>::Error>,
    {
        self.log(LogContent::new(
            &<T as TryInto<String>>::try_into(text)?,
            false,
        ))
    }
    pub fn error<T>(&self, text: T) -> Result<()>
    where
        T: TryInto<String>,
        Error: From<<T as TryInto<String>>::Error>,
    {
        self.log(LogContent::new(
            &<T as TryInto<String>>::try_into(text)?,
            true,
        ))
    }
}

#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)+) => {
        $logger.info(format_args!($($arg)+).to_string()).ok()
    };
}

#[macro_export]
macro_rules! error {
    ($logger:expr, $($arg:tt)+) => {
        $logger.error(format_args!($($arg)+).to_string()).ok()
    };
}
