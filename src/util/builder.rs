use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle, SelectMenu, SelectMenuOption, TextInput, TextInputStyle,
};
use twilight_model::channel::message::{Component, ReactionType};

/// Create an action row with a builder.
#[must_use = "must be built into an action row"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRowBuilder(ActionRow);

impl ActionRowBuilder {
    /// Creates a new action row builder.
    pub fn new(components: impl IntoIterator<Item = impl Into<Component>>) -> Self {
        let components = components.into_iter().take(5).map(Into::into).collect();

        Self(ActionRow { components })
    }

    /// Build into an action row.
    #[inline]
    #[must_use = "should be used as part of a component"]
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

/// Create a button with a builder.
#[must_use = "must be built into a button"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ButtonBuilder(Button);

impl ButtonBuilder {
    /// Creates a new button builder.
    #[inline]
    pub fn new(style: impl Into<ButtonStyle>) -> Self {
        Self(Button { custom_id: None, disabled: false, emoji: None, label: None, style: style.into(), url: None })
    }

    /// Add a custom identifier.
    pub fn custom_id(mut self, custom_id: impl Into<String>) -> Self {
        self.0.custom_id.replace(custom_id.into());

        self
    }

    /// Sets whether the button is disabled.
    pub fn disabled(mut self, disabled: impl Into<bool>) -> Self {
        self.0.disabled = disabled.into();

        self
    }

    /// Add an emoji.
    pub fn emoji(mut self, emoji: impl Into<ReactionType>) -> Self {
        self.0.emoji = Some(emoji.into());

        self
    }

    /// Add a label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.0.label = Some(label.into());

        self
    }

    /// Add a URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.0.url = Some(url.into());

        self
    }

    /// Build into a button.
    #[inline]
    #[must_use = "should be used as part of a component"]
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

/// Create a select menu with a builder.
#[must_use = "must be built into a select menu"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectMenuBuilder(SelectMenu);

impl SelectMenuBuilder {
    /// Creates a new select menu builder.
    #[inline]
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
    pub fn disabled(mut self, disabled: impl Into<bool>) -> Self {
        self.0.disabled = disabled.into();

        self
    }

    /// Add maximum values.
    pub fn max_values(mut self, max_values: impl Into<u8>) -> Self {
        self.0.max_values = Some(max_values.into());

        self
    }

    /// Add minimum values.
    pub fn min_values(mut self, min_values: impl Into<u8>) -> Self {
        self.0.min_values = Some(min_values.into());

        self
    }

    /// Add an option.
    pub fn option(mut self, option: impl Into<SelectMenuOption>) -> Self {
        self.0.options.push(option.into());

        self
    }

    /// Add a placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0.placeholder = Some(placeholder.into());

        self
    }

    /// Build into a select menu.
    #[inline]
    #[must_use = "should be used as part of a component"]
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

/// Create a select menu option with a builder.
#[must_use = "must be built into a select menu option"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectMenuOptionBuilder(SelectMenuOption);

impl SelectMenuOptionBuilder {
    /// Creates a new select menu option builder.
    #[inline]
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
    pub fn default(mut self, default: impl Into<bool>) -> Self {
        self.0.default = default.into();

        self
    }

    /// Add a description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.0.description = Some(description.into());

        self
    }

    /// Add an emoji.
    pub fn emoji(mut self, emoji: impl Into<ReactionType>) -> Self {
        self.0.emoji = Some(emoji.into());

        self
    }

    /// Build into a select menu option.
    #[inline]
    #[must_use = "should be used as part of a select menu"]
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

/// Create a text input with a builder.
#[must_use = "must be built into a text input"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputBuilder(TextInput);

impl TextInputBuilder {
    /// Creates a new text input builder.
    #[inline]
    pub fn new(custom_id: impl Into<String>, label: impl Into<String>, style: impl Into<TextInputStyle>) -> Self {
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
    pub fn max_length(mut self, max_length: impl Into<u16>) -> Self {
        self.0.max_length = Some(max_length.into());

        self
    }

    /// Add a minimum length.
    pub fn min_length(mut self, min_length: impl Into<u16>) -> Self {
        self.0.min_length = Some(min_length.into());

        self
    }

    /// Add a placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.0.placeholder = Some(placeholder.into());

        self
    }

    /// Set whether the text input is required.
    pub fn required(mut self, required: impl Into<bool>) -> Self {
        self.0.required = Some(required.into());

        self
    }

    /// Add a value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.0.value = Some(value.into());

        self
    }

    /// Build into a text input.
    #[inline]
    #[must_use = "should be used as part of a component"]
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
