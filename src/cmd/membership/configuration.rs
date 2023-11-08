use anyhow::bail;
use doop_localizer::{localize, Locale};
use doop_macros::Storage;
use doop_storage::{Stored, Toml, Value};
use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{ActionRow, ButtonStyle, TextInputStyle};
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::{Component, Embed, ReactionType};
use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::client::ApiRef;
use crate::cmd::membership::submission::Submission;
use crate::cmd::{CommandEntry, CommandOptionResolver};
use crate::util::builder::{ButtonBuilder, Modal, ModalBuilder, TextInputBuilder};
use crate::util::extension::{EmbedAuthorExtension, ReactionTypeExtension};
use crate::util::traits::{IntoImageSource, PreferLocale};
use crate::util::{Anchor, DataId, Result, BRANDING};

/// A guild's membership configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Storage)]
#[format(Toml)]
#[location("{}/{}/config", &'static str, Id<GuildMarker>)]
pub struct Config {
    /// The guild's identifier.
    pub id: Id<GuildMarker>,
    /// The guild's message anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    /// The guild's entrypoint configuration.
    pub entrypoint: ConfigEntrypoint,
    /// The guild's submission configuration.
    pub submission: ConfigSubmission,
}

impl Config {
    /// Creates a new configuration from the given option resolver.
    ///
    /// # Errors
    ///
    /// This function will return an error if a required argument is missing.
    pub fn new(id: Id<GuildMarker>, resolver: &CommandOptionResolver) -> Result<Self> {
        let entrypoint = ConfigEntrypoint::try_from(resolver)?;
        let submission = ConfigSubmission::try_from(resolver)?;

        Ok(Self { id, anchor: None, entrypoint, submission })
    }

    /// Builds the guild's entrypoint.
    ///
    /// # Errors
    ///
    /// This function will return an error if the entrypoint could not be constructed.
    pub async fn build_entrypoint(
        &self,
        entry: &CommandEntry,
        api: ApiRef<'_>,
    ) -> Result<(Embed, Vec<Component>)> {
        let guild = api.http.guild(self.id).await?.model().await?;

        let embed = EmbedBuilder::new()
            .author(EmbedAuthor::parse(&guild)?)
            .color(BRANDING)
            .description(&(*self.entrypoint.description))
            .thumbnail((&guild).into_image_source()?)
            .title(&(*self.entrypoint.title));

        let locale = guild.preferred_locale();

        let apply = ButtonBuilder::new(ButtonStyle::Primary)
            .custom_id(DataId::new(entry.name, "apply"))
            .disabled(!self.entrypoint.open)
            .emoji(ReactionType::parse('👋')?)
            .label(localize!(try in locale, "button.{}.apply.label", entry.name));
        let about = ButtonBuilder::new(ButtonStyle::Secondary)
            .custom_id(DataId::new(entry.name, "about"))
            .disabled(!self.entrypoint.open)
            .emoji(ReactionType::parse('🤔')?)
            .label(localize!(try in locale, "button.{}.about.label", entry.name));
        let row = ActionRow { components: vec![apply.into(), about.into()] };

        Ok((embed.build(), vec![Component::ActionRow(row)]))
    }

    /// Builds the guild's application modal.
    ///
    /// # Errors
    ///
    /// This function will return an error if the modal could not be constructed.
    pub fn build_application(
        &self,
        entry: &CommandEntry,
        user_id: Id<UserMarker>,
        locale: Locale,
    ) -> Result<Modal> {
        let title = localize!(try in locale, "modal.{}.application.title", entry.name);
        let custom_id = DataId::new(entry.name, "application");
        let mut modal = ModalBuilder::new(custom_id, title);

        let past = Submission::stored((entry.name, self.id, user_id));
        let submission = past.read().ok().map(Value::get_owned);

        for (index, question) in self.submission.questions.iter().enumerate() {
            let index = index.to_string();
            let mut input = TextInputBuilder::new(index, &(**question), TextInputStyle::Paragraph);

            if let Some(answer) = submission
                .as_ref()
                .and_then(|s| s.answers.iter().find_map(|(q, a)| (q == question).then_some(a)))
            {
                input = input.value(&(**answer));
            }

            modal.push(input.max_length(512).required(true))?;
        }

        Ok(modal.build())
    }
}

/// A guild's membership entrypoint configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigEntrypoint {
    /// The title of the entrypoint embed.
    pub title: Box<str>,
    /// The description of the entrypoint embed.
    pub description: Box<str>,
    /// Whether submissions are currently open.
    pub open: bool,
}

impl TryFrom<&CommandOptionResolver<'_>> for ConfigEntrypoint {
    type Error = anyhow::Error;

    fn try_from(resolver: &CommandOptionResolver<'_>) -> std::result::Result<Self, Self::Error> {
        let title = Box::from(resolver.get_str("title")?);
        let description = Box::from(resolver.get_str("description")?);

        Ok(Self { title, description, open: true })
    }
}

/// A guild's membership submission configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSubmission {
    /// The guild's submission output channel identifier.
    pub output_channel_id: Id<ChannelMarker>,
    /// The guild's membership role identifier.
    pub member_role_id: Id<RoleMarker>,
    /// The application's questions
    pub questions: Box<[Box<str>]>,
}

impl TryFrom<&CommandOptionResolver<'_>> for ConfigSubmission {
    type Error = anyhow::Error;

    fn try_from(resolver: &CommandOptionResolver<'_>) -> Result<Self, Self::Error> {
        let output_channel_id = *resolver.get_channel_id("output_channel")?;
        let member_role_id = *resolver.get_role_id("member_role")?;
        let questions = (1_u8 ..= 5_u8)
            .map(|n| format!("question_{n}"))
            .filter_map(|s| resolver.get_str(&s).ok().map(Box::from))
            .collect::<Box<_>>();

        if questions.is_empty() {
            bail!("at least one question must be provided");
        }

        Ok(Self { output_channel_id, member_role_id, questions })
    }
}
