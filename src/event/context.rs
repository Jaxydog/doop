use time::{OffsetDateTime, UtcOffset};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::application::interaction::Interaction;

use crate::extend::IdExt;
use crate::utility::DataId;

/// A context containing command data.
pub type CommandContext<'ctx> = Context<'ctx, &'ctx CommandData>;
/// A context containing component data.
pub type ComponentContext<'ctx> = Context<'ctx, (&'ctx MessageComponentInteractionData, DataId)>;
/// A context containing modal data.
pub type ModalContext<'ctx> = Context<'ctx, (&'ctx ModalInteractionData, DataId)>;

/// An interaction event context.
#[derive(Clone, Copy, Debug)]
pub struct Context<'ctx, T: Send + Sync> {
    /// The context's data.
    pub data: T,
    /// The context's interaction event.
    pub event: &'ctx Interaction,
    /// A reference to the bot's HTTP client.
    http: &'ctx Client,
    /// A reference to the bot's in-memory cache.
    cache: &'ctx InMemoryCache,
}

impl<'ctx, T: Send + Sync> Context<'ctx, T> {
    /// Creates a new event context.
    #[inline]
    pub const fn new(
        data: T,
        event: &'ctx Interaction,
        http: &'ctx Client,
        cache: &'ctx InMemoryCache,
    ) -> Self {
        Self { data, event, http, cache }
    }

    /// Returns the context's interaction token.
    #[inline]
    pub const fn token(&self) -> &String { &self.event.token }

    /// Returns the context's interaction client.
    #[inline]
    pub const fn client(&self) -> InteractionClient {
        self.http.interaction(self.event.application_id)
    }

    /// Returns the context's interaction's users's preferred locale.
    #[inline]
    pub fn locale(&self) -> Option<&str> {
        self.event.user.as_ref().and_then(|u| u.locale.as_deref())
    }

    /// Returns the context's interaction identifier's creation date.
    #[inline]
    pub fn created_at(&self) -> OffsetDateTime { self.event.id.created_at() }

    /// Returns the context's interaction identifier's creation date in the
    /// given UTC offset.
    #[inline]
    pub fn created_at_in(&self, offset: impl Into<UtcOffset>) -> OffsetDateTime {
        self.event.id.created_at_in(offset)
    }
}

/// A cached HTTP value.
pub trait CachedHttp: Send + Sync {
    /// The value's associated HTTP client reference.
    fn http(&self) -> &Client;
    /// The value's associated in-memory cache reference.
    fn cache(&self) -> &InMemoryCache;
}

impl CachedHttp for (&Client, &InMemoryCache) {
    #[inline]
    fn http(&self) -> &Client { self.0 }

    #[inline]
    fn cache(&self) -> &InMemoryCache { self.1 }
}

impl CachedHttp for (&InMemoryCache, &Client) {
    #[inline]
    fn http(&self) -> &Client { self.1 }

    #[inline]
    fn cache(&self) -> &InMemoryCache { self.0 }
}

impl<T: Send + Sync> CachedHttp for Context<'_, T> {
    #[inline]
    fn http(&self) -> &Client { self.http }

    #[inline]
    fn cache(&self) -> &InMemoryCache { self.cache }
}
