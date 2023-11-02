use std::sync::Arc;

use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;

/// The bot's HTTP and cache APIs.
#[derive(Clone, Debug)]
pub struct Api {
    /// The API's HTTP client.
    pub http: Arc<Client>,
    /// The API's in-memory cache.
    pub cache: Arc<InMemoryCache>,
}

impl Api {
    /// Returns a shared reference to this [`Api`].
    #[must_use]
    pub const fn into_ref(&self) -> ApiRef {
        ApiRef { http: &self.http, cache: &self.cache }
    }

    /// Returns an exclusive reference to this [`Api`].
    #[must_use]
    pub fn into_mut(&mut self) -> ApiMut {
        ApiMut { http: &mut self.http, cache: &mut self.cache }
    }
}

/// A reference to the bot's HTTP and cache APIs.
#[derive(Clone, Copy, Debug)]
pub struct ApiRef<'api> {
    /// The API's HTTP client.
    pub http: &'api Arc<Client>,
    /// The API's in-memory cache.
    pub cache: &'api Arc<InMemoryCache>,
}

impl<'api> ApiRef<'api> {
    /// Returns an owned clone of this [`ApiRef`].
    #[must_use]
    pub fn into_owned(&self) -> Api {
        Api { http: Arc::clone(self.http), cache: Arc::clone(self.cache) }
    }
}

/// A mutable reference to the bot's HTTP and cache APIs.
#[derive(Debug)]
pub struct ApiMut<'api> {
    /// The API's HTTP client.
    pub http: &'api mut Arc<Client>,
    /// The API's in-memory cache.
    pub cache: &'api mut Arc<InMemoryCache>,
}

impl<'api> ApiMut<'api> {
    /// Returns an owned clone of this [`ApiMut`].
    #[must_use]
    pub fn into_owned(&self) -> Api {
        Api { http: Arc::clone(self.http), cache: Arc::clone(self.cache) }
    }
}
