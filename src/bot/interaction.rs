use std::fmt::Display;

use doop_localizer::{localize, Locale};
use twilight_http::client::InteractionClient;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::application::interaction::modal::ModalInteractionData;
use twilight_model::application::interaction::Interaction;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::client::ApiRef;
use crate::util::{Result, BRANDING, FAILURE, SUCCESS};

/// A command interaction event context.
pub type CommandCtx<'api, 'evt> = Ctx<'api, 'evt, &'evt CommandData>;

/// A component interaction event context.
pub type ComponentCtx<'api, 'evt> = Ctx<'api, 'evt, &'evt MessageComponentInteractionData>;

/// A modal interaction event context.
pub type ModalCtx<'api, 'evt> = Ctx<'api, 'evt, &'evt ModalInteractionData>;

/// An interaction event context.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'api: 'evt, 'evt, T: Send> {
    /// The HTTP and cache APIs.
    pub api: ApiRef<'api>,
    /// The referenced interaction event.
    pub event: &'evt Interaction,
    /// The data of this interaction context.
    pub data: T,
    /// Whether the event has been deferred.
    defer_state: Option<bool>,
}

impl<'api: 'evt, 'evt, T: Send> Ctx<'api, 'evt, T> {
    /// Creates a new interaction event [`Ctx<T>`].
    pub const fn new(api: ApiRef<'api>, event: &'evt Interaction, data: T) -> Self {
        Self { api, event, data, defer_state: None }
    }

    /// Returns the interaction client of this interaction event [`Ctx<T>`].
    pub fn client(&self) -> InteractionClient {
        self.api.http.interaction(self.event.application_id)
    }

    /// Defers the interaction.
    ///
    /// # Errors
    ///
    /// This function will return an error if responding failed.
    pub async fn defer(&mut self, ephemeral: bool) -> Result {
        if self.defer_state.is_some() {
            return Ok(());
        }

        if ephemeral {
            crate::respond!(as self => {
                let kind = DeferredChannelMessageWithSource;
                let flags = EPHEMERAL;
            })
            .await?;
        } else {
            crate::respond!(as self => {
                let kind = DeferredChannelMessageWithSource;
            })
            .await?;
        }

        self.defer_state = Some(ephemeral);

        Ok(())
    }

    /// Defers the component or modal interaction.
    ///
    /// # Errors
    ///
    /// This function will return an error if responding failed or this is used for a non-modal or
    /// non-component interaction.
    pub async fn defer_update(&mut self, ephemeral: bool) -> Result {
        if self.defer_state.is_some() {
            return Ok(());
        }

        if ephemeral {
            crate::respond!(as self => {
                let kind = DeferredUpdateMessage;
                let flags = EPHEMERAL;
            })
            .await?;
        } else {
            crate::respond!(as self => {
                let kind = DeferredUpdateMessage;
            })
            .await?;
        }

        self.defer_state = Some(ephemeral);

        Ok(())
    }

    /// Responds to the interaction with a message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be responded to.
    async fn complete(
        self,
        locale: Locale,
        group: &str,
        key: impl Display + Send,
        color: u32,
        has_desc: bool,
    ) -> Result {
        let title = localize!(try in locale, "{group}.{key}.title");
        let mut embed = EmbedBuilder::new().color(color).title(title);

        if has_desc {
            let description = localize!(try in locale, "{group}.{key}.description");

            embed = embed.description(description);
        }

        match self.defer_state {
            Some(false) => {
                crate::followup!(as self => {
                    let embeds = &[embed.build()];
                })
                .await?;
            }
            Some(true) => {
                crate::followup!(as self => {
                    let embeds = &[embed.build()];
                    let flags = EPHEMERAL;
                })
                .await?;
            }
            None => {
                crate::respond!(as self => {
                    let kind = ChannelMessageWithSource;
                    let embeds = [embed.build()];
                    let flags = EPHEMERAL;
                })
                .await?;
            }
        }

        Ok(())
    }

    /// Responds to the interaction with a success message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be responded to.
    #[inline]
    pub async fn success(self, locale: Locale, key: impl Display + Send, has_desc: bool) -> Result {
        self.complete(locale, "success", key, SUCCESS, has_desc).await
    }

    /// Responds to the interaction with a notification message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be responded to.
    #[inline]
    pub async fn notify(self, locale: Locale, key: impl Display + Send, has_desc: bool) -> Result {
        self.complete(locale, "notify", key, BRANDING, has_desc).await
    }

    /// Responds to the interaction with a failure message.
    ///
    /// # Errors
    ///
    /// This function will return an error if the interaction could not be responded to.
    #[inline]
    pub async fn failure(self, locale: Locale, key: impl Display + Send, has_desc: bool) -> Result {
        self.complete(locale, "failure", key, FAILURE, has_desc).await
    }
}

/// Responds to an interaction event.
///
/// # Examples
///
/// ```
/// respond!(as api.http, event => {
///     let kind = DeferredChannelMessageWithSource;
///     let embeds = &[embed.build()];
/// })
/// .await?;
///
/// respond!(as ctx => {
///     let kind = DeferredChannelMessageWithSource;
///     let embeds = &[embed.build()];
/// })
/// .await?;
/// ```
#[macro_export]
macro_rules! respond {
    (as $http:expr, $event:expr => { $($args:tt)+ }) => {
        $crate::respond!(@($http.interaction($event.application_id), $event.id, &$event.token, { $($args)+ }))
    };
    (as $ctx:expr => { $($args:tt)+ }) => {
        $crate::respond!(@($ctx.client(), $ctx.event.id, &$ctx.event.token, { $($args)+ }))
    };
    (@($client:expr, $id:expr, $token:expr, {
        let kind = $kind:ident;
        $(let attachments = $attachments:expr;)?
        $(let choices = $choices:expr;)?
        $(let components = $components:expr;)?
        $(let content = $content:expr;)?
        $(let custom_id = $custom_id:expr;)?
        $(let embeds = $embeds:expr;)?
        $(let flags = $($flag:ident)|+;)?
        $(let mentions = { $($mentions:tt)+ })?
        $(let title = $title:expr;)?
        $(let tts = $tts:literal;)?
    })) => {
        $client.create_response($id, $token, &::twilight_model::http::interaction::InteractionResponse {
            kind: ::twilight_model::http::interaction::InteractionResponseType::$kind,
            data: Some(
                ::twilight_util::builder::InteractionResponseDataBuilder::new()
                    $(.attachments($attachments))?
                    $(.choices($choices))?
                    $(.components($components))?
                    $(.content($content))?
                    $(.custom_id($custom_id))?
                    $(.embeds($embeds))?
                    $(.flags(::twilight_model::channel::message::MessageFlags::empty()$(.union(::twilight_model::channel::message::MessageFlags::$flag))+))?
                    $(.allowed_mentions(::twilight_model::channel::message::AllowedMentions { $($mentions)+ }))?
                    $(.title($title))?
                    $(.tts($tts))?
                    .build()
            ),
        })
    };
}

/// Follows-up an interaction event response.
///
/// # Examples
///
/// ```
/// followup!(as api.http, event => {
///     let embeds = &[embed.build()];
///     let flags = EPHEMERAL;
/// })
/// .await?;
///
/// followup!(as ctx => {
///     let embeds = &[embed.build()];
///     let flags = EPHEMERAL;
/// })
/// .await?;
/// ```
#[macro_export]
macro_rules! followup {
    (as $http:expr, $event:expr => { $($args:tt)* }) => {
        $crate::followup!(@($http.interaction($event.application_id), &$event.token, { $($args)* }))
    };
    (as $ctx:expr => { $($args:tt)* }) => {
        $crate::followup!(@($ctx.client(), &$ctx.event.token, { $($args)* }))
    };
    (@($client:expr, $token:expr, {
        $(let attachments = $attachments:expr;)?
        $(let components = $components:expr;)?
        $(let content = $content:expr;)?
        $(let embeds = $embeds:expr;)?
        $(let flags = $($flag:ident)|+;)?
        $(let mentions = { $($mentions:tt)+ })?
        $(let tts = $tts:literal;)?
    })) => {
        $client.create_followup($token)
            $(.attachments($attachments)?)?
            $(.components($components)?)?
            $(.content($content)?)?
            $(.embeds($embeds)?)?
            $(.flags(::twilight_model::channel::message::MessageFlags::empty()$(.union(::twilight_model::channel::message::MessageFlags::$flag))+))?
            $(.allowed_mentions(::twilight_model::channel::message::AllowedMentions { $($mentions)+ }))?
            $(.tts($tts))?
    };
}
