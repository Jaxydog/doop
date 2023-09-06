#![doc = env!("CARGO_PKG_DESCRIPTION")]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use std::path::PathBuf;

use doop_logger::{info, Flags};
use futures_util::future::{select, Either};
use futures_util::pin_mut;
use tokio::runtime::Builder;

use crate::bot::BotClient;
use crate::util::{Arguments, Result};

/// Defines and implements the bot client and event handling.
pub mod bot;
/// Defines and implements the bot's commands and interaction handlers.
pub mod cmd;
/// Defines and implements commonly-used utilities.
pub mod util;

/// The bot application entrypoint.
///
/// # Errors
///
/// This function will return an error if the process encounters a fatal error.
pub fn main() -> Result {
    let arguments = crate::util::arguments();

    install_logger(arguments)?;
    install_localizer(arguments)?;
    install_storage(arguments)?;

    dotenvy::dotenv()?;
    info!("loaded environment variables")?;

    Builder::new_multi_thread().enable_all().build()?.block_on(application())
}

/// The main asynchronous application function.
///
/// # Errors
///
/// This function will return an error if the process encounters a fatal error.
pub async fn application() -> Result {
    info!("initialized asynchronous runtime")?;

    let client = BotClient::new().await?;
    info!("initialized client instance")?;

    let process = client.start();
    let terminate = tokio::signal::ctrl_c();
    info!("started client process")?;

    pin_mut!(process);
    pin_mut!(terminate);

    match select(process, terminate).await {
        Either::Left(_) => (),
        Either::Right(_) => info!("received abort signal")?,
    }

    Ok(info!("stopped client process")?)
}

/// Installs the logger instance.
///
/// # Errors
///
/// This function will return an error if the logger could not output the installation notice log.
fn install_logger(arguments: &Arguments) -> Result {
    let Arguments { log_no_print, log_no_write, log_no_error, log_write_dir, log_error_dir, .. } = arguments;

    let flags = Flags { no_print: *log_no_print, no_write: *log_no_write, no_error: *log_no_error };
    let write = log_write_dir.clone().unwrap_or_else(|| PathBuf::from("logs"));
    let error = log_error_dir.clone().unwrap_or_else(|| PathBuf::from("logs/.error"));

    doop_logger::install_into(flags, write, error);

    Ok(info!("initialized logger instance")?)
}

/// Installs the localizer instance.
///
/// # Errors
///
/// This function will return an error if the logger could not output the installation notice log.
fn install_localizer(arguments: &Arguments) -> Result {
    let Arguments { lang_prefer_locale, lang_file_dir, .. } = arguments;

    let preferred = lang_prefer_locale.unwrap_or_default();
    let directory = lang_file_dir.clone().unwrap_or_else(|| PathBuf::from("lang"));

    doop_localizer::install_into(preferred, directory);

    Ok(info!("initialized localizer instance")?)
}

/// Installs the storage directory.
///
/// # Errors
///
/// This function will return an error if the logger could not output the installation notice log.
fn install_storage(arguments: &Arguments) -> Result {
    let Arguments { data_file_dir, .. } = arguments;

    let directory = data_file_dir.clone().unwrap_or_else(|| PathBuf::from("data"));

    doop_storage::install_into(directory);

    Ok(info!("initialized storage directory")?)
}
