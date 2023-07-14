use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;
use twilight_model::channel::message::{Component, Embed};

use crate::utility::{Modal, Result};

/// Implements a method for building embeds.
pub trait SyncEmbedBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds embeds using the provided arguments.
    fn build_embeds(&self, arguments: Self::Arguments) -> Result<Vec<Embed>>;
}

/// Implements a method for building embeds.
#[async_trait::async_trait]
pub trait AsyncEmbedBuilder: Sync {
    /// The arguments passed into the builder method.
    type Arguments: Send + Sync;

    /// Builds embeds using the provided arguments.
    async fn build_embeds(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        arguments: Self::Arguments,
    ) -> Result<Vec<Embed>>;
}

/// Implements a method for building modals.
pub trait SyncModalBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds modals using the provided arguments.
    fn build_modals(&self, arguments: Self::Arguments) -> Result<Vec<Modal>>;
}

/// Implements a method for building modals.
#[async_trait::async_trait]
pub trait AsyncModalBuilder: Sync {
    /// The arguments passed into the builder method.
    type Arguments: Send + Sync;

    /// Builds modals using the provided arguments.
    async fn build_modals(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        arguments: Self::Arguments,
    ) -> Result<Vec<Modal>>;
}

/// Implements a method for building button components.
pub trait SyncButtonBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds button components using the provided arguments.
    fn build_buttons(&self, disabled: bool, arguments: Self::Arguments) -> Result<Vec<Component>>;
}

/// Implements a method for building button components.
#[async_trait::async_trait]
pub trait AsyncButtonBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds button components using the provided arguments.
    async fn build_buttons(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        disabled: bool,
        arguments: Self::Arguments,
    ) -> Result<Vec<Component>>;
}

/// Implements a method for building input components.
pub trait SyncInputBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds input components using the provided arguments.
    fn build_inputs(&self, arguments: Self::Arguments) -> Result<Vec<Component>>;
}

/// Implements a method for building input components.
#[async_trait::async_trait]
pub trait AsyncInputBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds input components using the provided arguments.
    async fn build_inputs(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        arguments: Self::Arguments,
    ) -> Result<Vec<Component>>;
}

/// Implements a method for building selector components.
pub trait SyncSelectorBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds selector components using the provided arguments.
    fn build_selectors(&self, arguments: Self::Arguments) -> Result<Vec<Component>>;
}

/// Implements a method for building selector components.
#[async_trait::async_trait]
pub trait AsyncSelectorBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds selector components using the provided arguments.
    async fn build_selectors(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        arguments: Self::Arguments,
    ) -> Result<Vec<Component>>;
}
