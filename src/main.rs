//! An open-source Discord server moderation bot.
#![deny(clippy::expect_used, unsafe_code, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::todo, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use std::path::PathBuf;

use doop_logger::{info, Config};
use futures_util::future::{select, Either};
use futures_util::pin_mut;
use tokio::runtime::Builder;

use crate::bot::BotClient;
use crate::util::{arguments, Arguments, Result};

/// Defines and implements the bot's client and event handlers.
pub mod bot;
/// Defines and implements the bot's commands.
pub mod cmd;
/// Defines and implements commonly-used utilities.
pub mod util;

/// The application's entrypoint.
///
/// # Errors
///
/// This function will return an error if the async runtime could not be initialized.
pub fn main() -> Result {
    let arguments = arguments();
    // this needs to be assigned to prevent the thread from joining immediately.
    let _lt = install_logger(arguments);

    info!("initialized logging thread")?;

    install_storage(arguments);

    info!("initialized storage directory")?;

    install_localizer(arguments);

    info!("initialized localizer instance")?;

    #[cfg(feature = "dotenv")]
    {
        dotenvy::dotenv()?;
        info!("loaded environment variables")?;
    }

    Builder::new_multi_thread().enable_all().build()?.block_on(main_async())
}

/// The application's (async) entrypoint.
///
/// # Panics
///
/// Panics if a thread could not be joined.
///
/// # Errors
///
/// This function will return an error if the application's execution fails.
#[allow(clippy::unwrap_used)]
pub async fn main_async() -> Result {
    info!("initialized asynchronous runtime")?;

    let client = BotClient::new().await?;
    info!("initialized client instance")?;

    let process = client.start();
    let termination = tokio::signal::ctrl_c();
    info!("started client process")?;

    pin_mut!(process);
    pin_mut!(termination);

    match select(process, termination).await {
        Either::Left(_) => (),
        Either::Right(_) => info!("received termination signal")?,
    }

    Ok(info!("stopped client process")?)
}

/// Installs the logger instance.
///
/// # Errors
///
/// This function will return an error if log(s) failed to output.
fn install_logger(arguments: &Arguments) -> std::io::Result<doop_logger::LogThread> {
    let dir = arguments.log_output_dir.clone().unwrap_or_else(|| PathBuf::from("log").into());
    let config = Config {
        print: !arguments.log_no_print,
        write: !arguments.log_no_write,
        color: !arguments.log_no_color,
        queue_size: arguments.log_queue_capacity.unwrap_or(8),
        stale_time: arguments.log_queue_timeout.unwrap_or(20),
    };

    doop_logger::install(config, dir)
}

/// Installs the storage directory.
fn install_storage(arguments: &Arguments) {
    let dir = arguments.data_dir.clone().unwrap_or_else(|| PathBuf::from("res").into());

    doop_storage::install(dir);
}

/// Installs the localizer instance.
fn install_localizer(arguments: &Arguments) {
    let dir = arguments.data_dir.clone().unwrap_or_else(|| PathBuf::from("res").into());
    let dir = arguments.l18n_map_dir.clone().unwrap_or_else(|| dir.join("lang").into());
    let prefer = arguments.l18n_prefer.unwrap_or(doop_localizer::Locale::EnglishUS);

    doop_localizer::install(prefer, dir);
}
