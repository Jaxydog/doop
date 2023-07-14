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
pub type CommandCtx<'ctx> = Ctx<'ctx, &'ctx CommandData>;
/// A context containing component data.
pub type ComponentCtx<'ctx> = Ctx<'ctx, (&'ctx MessageComponentInteractionData, DataId)>;
/// A context containing modal data.
pub type ModalCtx<'ctx> = Ctx<'ctx, (&'ctx ModalInteractionData, DataId)>;

/// An interaction event context.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'ctx, T> {
    /// The bot's HTTP client.
    pub http: &'ctx Client,
    /// The bot's in-memory cache.
    pub cache: &'ctx InMemoryCache,
    /// The bot's event interaction.
    pub interaction: &'ctx Interaction,
    /// The bot's interaction data.
    pub data: T,
}

impl<'ctx, T> Ctx<'ctx, T> {
    /// Returns the context's interaction token.
    #[inline]
    pub const fn token(&self) -> &String {
        &self.interaction.token
    }

    /// Returns the context's interaction client.
    #[inline]
    pub const fn client(&self) -> InteractionClient {
        self.http.interaction(self.interaction.application_id)
    }

    /// Returns the context's user locale.
    #[inline]
    #[must_use]
    pub fn locale(&self) -> Option<&str> {
        self.interaction
            .user
            .as_ref()
            .and_then(|u| u.locale.as_deref())
    }

    /// Returns the context interaction identifier's creation date.
    #[inline]
    #[must_use]
    pub fn created_at(&self) -> OffsetDateTime {
        self.interaction.id.created_at()
    }

    /// Returns the context interaction identifier's creation date in the given
    /// UTC offset.
    #[inline]
    #[must_use]
    pub fn created_at_in(&self, offset: impl Into<UtcOffset>) -> OffsetDateTime {
        self.interaction.id.created_at_in(offset)
    }
}
