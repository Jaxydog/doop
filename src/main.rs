//! Doop! An open-source Discord guild moderation bot.
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::cargo, clippy::nursery, clippy::pedantic)]
#![warn(clippy::todo, missing_docs)]
#![allow(clippy::module_name_repetitions, clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::sync::Arc;
use std::time::Duration;

use serenity::client::Cache;
use serenity::http::Http;
use serenity::Client;
use tokio::spawn;
use tokio::time::interval;
use util::Result;

use crate::event::EventHandler;
use crate::util::{get_arguments, get_bot_token, BOT_INTENTS};

/// Contains common code for commands and all command definitions
pub mod cmd;
/// Provides common structures and traits
pub mod common;
/// Provides definitions for the bot's event handler
pub mod event;
/// Contains useful functions, macros, and definitions
pub mod util;

/// Bot process entrypoint
#[tokio::main]
pub async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    info!("initializing client...");

    let mut client = Client::builder(get_bot_token()?, BOT_INTENTS)
        .event_handler(EventHandler)
        .await?;

    tokio::spawn(function_loop_daemon());

    Ok(client.start_autosharded().await?)
}

/// Ensures that `function_loop` continues running forever
pub async fn function_loop_daemon() -> ! {
    let delay = get_arguments().function_loop_delay;

    info!("starting function loop daemon");

    loop {
        if let Err(error) = spawn(function_loop(delay)).await {
            error!("function loop encountered an error: {error}");
        }
    }
}

/// Runs an inner loop every `seconds` seconds concurrently
pub async fn function_loop(seconds: u64) -> Result<()> {
    let mut interval = interval(Duration::from_secs(seconds));

    let cache = Arc::new(Cache::new());
    let http = Http::new(&get_bot_token()?);
    let _http = (&cache, &http);

    loop {
        interval.tick().await;
    }
}
