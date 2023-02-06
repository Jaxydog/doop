#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::cargo, clippy::nursery, clippy::pedantic, clippy::todo)]
#![allow(clippy::module_name_repetitions)]
#![feature(iter_array_chunks)]

use clap::Parser;
use serenity::Client;

use crate::{handler::Handler, prelude::*};

pub(crate) mod command;
pub(crate) mod handler;
pub(crate) mod prelude;
pub(crate) mod utility;

#[derive(Debug, Parser)]
#[command(about, author)]
struct Arguments {
    /// Disable logger console output
    #[arg(long)]
    pub no_log: bool,
    /// Disable logger file output
    #[arg(long, short)]
    pub no_save: bool,
    /// The number of seconds between function loop ticks
    #[arg(long, short, default_value = "10")]
    pub interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let arguments = Arguments::parse();
    let token = get_token()?;
    let logger = Logger::new(arguments.no_log, !arguments.no_save)?;
    let handler = Handler::new(logger.clone());

    info!(logger, "Starting client...");

    let mut client = Client::builder(&token, INTENTS)
        .event_handler(handler)
        .await?;

    tokio::spawn(function_loop(logger, arguments.interval, token));
    client.start_autosharded().await.map_err(Into::into)
}

async fn function_loop(logger: Logger, interval: u64, token: String) -> ! {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval));
    let cache = std::sync::Arc::new(serenity::cache::Cache::new());
    let http = serenity::http::Http::new(&token);

    info!(logger, "Function loop started!");

    loop {
        let _http = (&cache, &http);

        interval.tick().await;
    }
}
