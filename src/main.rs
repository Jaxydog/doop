//! An open-source Discord guild moderation bot.
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]
#![feature(control_flow_enum, try_blocks, try_trait_v2, try_trait_v2_residual)]

/// Contains command definitions and implementations.
pub mod command;
/// Contains event handler definitions and content.
pub mod event;
/// Provides trait extensions for various types.
pub mod extend;
/// Provides an interface for the localization of bot content.
pub mod locale;
/// Provides an interface for logging to the console and file system.
pub mod logger;
/// Defines commonly-used macro definitions.
pub mod macros;
/// Defines and implements the bot process.
pub mod state;
/// Provides traits and structures for reading and writing bot data.
pub mod storage;
/// Defines commonly-used trait definitions.
pub mod traits;
/// Contains various commonly-used definitions.
pub mod utility;

/// The bot process' entrypoint.
#[tokio::main]
pub async fn main() -> self::utility::Result {
    dotenvy::dotenv()?;

    info!("initializing bot state...")?;

    let state = state::State::new().await?;

    info!("starting bot process...")?;

    let process = state.run();
    let exit = tokio::signal::ctrl_c();

    futures_util::pin_mut!(process);
    futures_util::pin_mut!(exit);

    match futures_util::future::select(process, exit).await {
        futures_util::future::Either::Left(_) => (),
        futures_util::future::Either::Right(_) => info!("received exit signal")?,
    }

    info!("stopping bot process...")
}
