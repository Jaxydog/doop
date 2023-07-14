use anyhow::anyhow;
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

/// Implements a method for building components.
pub trait SyncComponentBuilder {
    /// The arguments passed into the builder method.
    type Arguments;

    /// Builds components using the provided arguments.
    fn build_components(&self, arguments: Self::Arguments) -> Result<Vec<Component>>;
}

/// Implements a method for building components.
#[async_trait::async_trait]
pub trait AsyncComponentBuilder: Sync {
    /// The arguments passed into the builder method.
    type Arguments: Send + Sync;

    /// Builds components using the provided arguments.
    async fn build_components(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        arguments: Self::Arguments,
    ) -> Result<Vec<Component>>;
}

/// Implements methods for multiple building components.
pub trait SyncMultiComponentBuilder {
    /// The arguments passed into the button builder method.
    type ButtonArgs;
    /// The arguments passed into the input builder method.
    type InputArgs;
    /// The arguments passed into the select builder method.
    type SelectArgs;

    /// Builds button components using the provided arguments.
    fn build_button_components(&self, _: Self::ButtonArgs) -> Result<Vec<Component>> {
        Err(anyhow!("unsupported builder method"))
    }
    /// Builds input components using the provided arguments.
    fn build_input_components(&self, _: Self::InputArgs) -> Result<Vec<Component>> {
        Err(anyhow!("unsupported builder method"))
    }
    /// Builds select components using the provided arguments.
    fn build_select_components(&self, _: Self::SelectArgs) -> Result<Vec<Component>> {
        Err(anyhow!("unsupported builder method"))
    }
}

/// Implements a method for building components.
#[async_trait::async_trait]
pub trait AsyncMultiComponentBuilder: Sync {
    /// The arguments passed into the button builder method.
    type ButtonArgs: Send + Sync;
    /// The arguments passed into the input builder method.
    type InputArgs: Send + Sync;
    /// The arguments passed into the select builder method.
    type SelectArgs: Send + Sync;

    /// Builds button components using the provided arguments.
    async fn build_button_components(
        &self,
        _http: &Client,
        _cache: &InMemoryCache,
        _: Self::ButtonArgs,
    ) -> Result<Vec<Component>> {
        Err(anyhow!("unsupported builder method"))
    }
    /// Builds input components using the provided arguments.
    async fn build_input_components(
        &self,
        _http: &Client,
        _cache: &InMemoryCache,
        _: Self::InputArgs,
    ) -> Result<Vec<Component>> {
        Err(anyhow!("unsupported builder method"))
    }
    /// Builds select components using the provided arguments.
    async fn build_select_components(
        &self,
        _http: &Client,
        _cache: &InMemoryCache,
        _: Self::SelectArgs,
    ) -> Result<Vec<Component>> {
        Err(anyhow!("unsupported builder method"))
    }
}
