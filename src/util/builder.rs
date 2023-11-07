use anyhow::bail;
use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle, SelectMenu, SelectMenuOption, TextInput, TextInputStyle,
};
use twilight_model::channel::message::{Component, ReactionType};

use crate::util::Result;

/// Creates an action row.
#[must_use = "must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRowBuilder(ActionRow);

impl ActionRowBuilder {
    /// Creates a new [`ActionRowBuilder`].
    pub const fn new() -> Self {
        Self(ActionRow { components: vec![] })
    }

    /// Appends an element to the action row.
    ///
    /// # Errors
    ///
    /// This function will return an error if the row contains five or more components.
    pub fn push(&mut self, component: impl Into<Component>) -> Result {
        if self.0.components.len() < 5 {
            self.0.components.push(component.into());

            Ok(())
        } else {
            bail!("a maximum of 5 components is supported");
        }
    }

    /// Builds the action row.
    #[inline]
    #[must_use]
    pub fn build(self) -> ActionRow {
        self.0
    }
}

impl From<ActionRowBuilder> for ActionRow {
    #[inline]
    fn from(value: ActionRowBuilder) -> Self {
        value.build()
    }
}

impl From<ActionRowBuilder> for Component {
    #[inline]
    fn from(value: ActionRowBuilder) -> Self {
        Self::ActionRow(value.build())
    }
}

impl<I: Into<Component>> FromIterator<I> for ActionRowBuilder {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let components = iter.into_iter().take(5).map(Into::into).collect();

        Self(ActionRow { components })
    }
}

/// Creates a button.
#[must_use = "must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ButtonBuilder(Button);

impl ButtonBuilder {
    /// Creates a new [`ButtonBuilder`].
    pub fn new(style: impl Into<ButtonStyle>) -> Self {
        Self(Button {
            custom_id: None,
            disabled: false,
            emoji: None,
            label: None,
            style: style.into(),
            url: None,
        })
    }

    /// Sets the button's custom identifier.
    pub fn custom_id(mut self, custom_id: impl Into<String>) -> Self {
        self.0.custom_id = Some(custom_id.into());

        self
    }

    /// Sets whether the button is disabled.
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.0.disabled = disabled;

        self
    }

    /// Sets the button's emoji.
    pub fn emoji(mut self, emoji: impl Into<ReactionType>) -> Self {
        self.0.emoji = Some(emoji.into());

        self
    }

    /// Sets the button's label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.0.label = Some(label.into());

        self
    }

    /// Sets the button's URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.0.url = Some(url.into());

        self
    }

    /// Builds the button.
    #[inline]
    #[must_use]
    pub fn build(self) -> Button {
        self.0
    }
}

impl From<ButtonBuilder> for Button {
    #[inline]
    fn from(value: ButtonBuilder) -> Self {
        value.build()
    }
}

impl From<ButtonBuilder> for Component {
    #[inline]
    fn from(value: ButtonBuilder) -> Self {
        Self::Button(value.build())
    }
}

/// Text input modal.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modal {
    /// The modal's custom identifier.
    pub custom_id: String,
    /// The modal's title.
    pub title: String,
    /// The modal's component list.
    pub components: Vec<Component>,
}

/// Creates a modal.
#[must_use = "must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalBuilder(Modal);

impl ModalBuilder {
    /// Creates a new [`ModalBuilder`].
    pub fn new(custom_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self(Modal { custom_id: custom_id.into(), title: title.into(), components: vec![] })
    }

    /// Appends an element to the action row.
    ///
    /// # Errors
    ///
    /// This function will return an error if the modal contains five or more components.
    pub fn push(&mut self, component: impl Into<TextInput>) -> Result {
        if self.0.components.len() >= 5 {
            bail!("a maximum of 5 components is supported");
        }

        let input = Component::TextInput(component.into());

        self.0.components.push(ActionRow { components: vec![input] }.into());

        Ok(())
    }

    /// Builds the modal.
    #[inline]
    #[must_use]
    pub fn build(self) -> Modal {
        self.0
    }
}

impl From<ModalBuilder> for Modal {
    #[inline]
    fn from(value: ModalBuilder) -> Self {
        value.build()
    }
}

/// Creates a selection menu.
#[must_use = "must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectMenuBuilder(SelectMenu);

impl SelectMenuBuilder {
    /// Creates a new [`SelectMenuBuilder`].
    pub fn new(custom_id: impl Into<String>) -> Self {
        Self(SelectMenu {
            custom_id: custom_id.into(),
            disabled: false,
            max_values: None,
            min_values: None,
            options: vec![],
            placeholder: None,
        })
    }

    /// Sets whether the menu is disabled.
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.0.disabled = disabled;

        self
    }

    /// Sets the maximum number of values within the menu.
    pub const fn max_values(mut self, max_values: u8) -> Self {
        self.0.max_values = Some(max_values);

        self
    }

    /// Sets the minimum number of values within the menu.
    pub const fn min_values(mut self, min_values: u8) -> Self {
        self.0.min_values = Some(min_values);

        self
    }

    /// Adds an option to the menu.
    pub fn option(mut self, option: impl Into<SelectMenuOption>) -> Self {
        self.0.options.push(option.into());

        self
    }

    /// Sets the menu's placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0.placeholder = Some(placeholder.into());

        self
    }

    /// Builds the select menu.
    #[inline]
    #[must_use]
    pub fn build(self) -> SelectMenu {
        self.0
    }
}

impl From<SelectMenuBuilder> for SelectMenu {
    #[inline]
    fn from(value: SelectMenuBuilder) -> Self {
        value.build()
    }
}

impl From<SelectMenuBuilder> for Component {
    #[inline]
    fn from(value: SelectMenuBuilder) -> Self {
        Self::SelectMenu(value.build())
    }
}

/// Creates a select menu option.
#[must_use = "must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectMenuOptionBuilder(SelectMenuOption);

impl SelectMenuOptionBuilder {
    /// Creates a new [`SelectMenuOptionBuilder`].
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self(SelectMenuOption {
            default: false,
            description: None,
            emoji: None,
            label: label.into(),
            value: value.into(),
        })
    }

    /// Sets whether the menu is the default.
    pub const fn default(mut self, default: bool) -> Self {
        self.0.default = default;

        self
    }

    /// Add a description to the option.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());

        self
    }

    /// Add an emoji to the option.
    pub fn emoji(mut self, emoji: impl Into<ReactionType>) -> Self {
        self.0.emoji = Some(emoji.into());

        self
    }

    /// Builds the select menu option.
    #[inline]
    #[must_use]
    pub fn build(self) -> SelectMenuOption {
        self.0
    }
}

impl From<SelectMenuOptionBuilder> for SelectMenuOption {
    #[inline]
    fn from(value: SelectMenuOptionBuilder) -> Self {
        value.build()
    }
}

/// Creates a text input.
#[must_use = "must be constructed"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputBuilder(TextInput);

impl TextInputBuilder {
    /// Creates a new text input builder.
    #[inline]
    pub fn new(
        custom_id: impl Into<String>,
        label: impl Into<String>,
        style: impl Into<TextInputStyle>,
    ) -> Self {
        Self(TextInput {
            custom_id: custom_id.into(),
            label: label.into(),
            max_length: None,
            min_length: None,
            placeholder: None,
            required: None,
            style: style.into(),
            value: None,
        })
    }

    /// Add a maximum length.
    pub const fn max_length(mut self, max_length: u16) -> Self {
        self.0.max_length = Some(max_length);

        self
    }

    /// Add a minimum length.
    pub const fn min_length(mut self, min_length: u16) -> Self {
        self.0.min_length = Some(min_length);

        self
    }

    /// Add a placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0.placeholder = Some(placeholder.into());

        self
    }

    /// Set whether the text input is required.
    pub const fn required(mut self, required: bool) -> Self {
        self.0.required = Some(required);

        self
    }

    /// Add a value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.0.value = Some(value.into());

        self
    }

    /// Builds the text input.
    #[inline]
    #[must_use]
    pub fn build(self) -> TextInput {
        self.0
    }
}

impl From<TextInputBuilder> for TextInput {
    #[inline]
    fn from(value: TextInputBuilder) -> Self {
        value.build()
    }
}

impl From<TextInputBuilder> for Component {
    #[inline]
    fn from(value: TextInputBuilder) -> Self {
        Self::TextInput(value.build())
    }
}
