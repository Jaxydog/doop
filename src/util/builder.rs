use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle, TextInput, TextInputStyle,
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
    pub fn build(self) -> ActionRow { self.0 }
}

impl From<ActionRowBuilder> for ActionRow {
    #[inline]
    fn from(value: ActionRowBuilder) -> Self { value.build() }
}

impl From<ActionRowBuilder> for Component {
    #[inline]
    fn from(value: ActionRowBuilder) -> Self { Self::ActionRow(value.build()) }
}

/// Create a button with a builder.
#[must_use = "must be built into a button"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ButtonBuilder(Button);

impl ButtonBuilder {
    /// Creates a new button builder.
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
    pub fn build(self) -> Button { self.0 }
}

impl From<ButtonBuilder> for Button {
    #[inline]
    fn from(value: ButtonBuilder) -> Self { value.build() }
}

impl From<ButtonBuilder> for Component {
    #[inline]
    fn from(value: ButtonBuilder) -> Self { Self::Button(value.build()) }
}

/// Create a text input with a builder.
#[must_use = "must be built into a text input"]
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputBuilder(TextInput);

impl TextInputBuilder {
    /// Creates a new text input builder.
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
    pub fn build(self) -> TextInput { self.0 }
}

impl From<TextInputBuilder> for TextInput {
    #[inline]
    fn from(value: TextInputBuilder) -> Self { value.build() }
}

impl From<TextInputBuilder> for Component {
    #[inline]
    fn from(value: TextInputBuilder) -> Self { Self::TextInput(value.build()) }
}
