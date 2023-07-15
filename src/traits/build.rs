use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;
use twilight_model::channel::message::component::{Button, SelectMenu, TextInput};
use twilight_model::channel::message::{Component, Embed};

use crate::utility::{ActionRowBuilder, Modal, Result};

/// A value that can build embeds using the provided arguments.
pub trait BuildEmbeds<A = (), R = Embed>
where
    R: Into<Embed>,
{
    /// Builds embeds using the provided arguments.
    fn build_embeds(&self, _: A) -> Result<Vec<R>>;
}

/// A value that can build embeds using the provided arguments.
#[async_trait::async_trait]
pub trait BuildEmbedsAsync<A = (), R = Embed>
where
    Self: Sync,
    A: Send + Sync,
    R: Into<Embed> + Send,
{
    /// Builds embeds using the provided arguments.
    async fn build_embeds(&self, http: &Client, cache: &InMemoryCache, _: A) -> Result<Vec<R>>;
}

/// A value that can build modals using the provided arguments.
pub trait BuildModals<A = (), R = Modal>
where
    R: Into<Modal>,
{
    /// Builds modals using the provided arguments.
    fn build_modals(&self, _: A) -> Result<Vec<R>>;
}

/// A value that can build modals using the provided arguments.
#[async_trait::async_trait]
pub trait BuildModalsAsync<A = (), R = Modal>
where
    Self: Sync,
    A: Send + Sync,
    R: Into<Modal> + Send,
{
    /// Builds modals using the provided arguments.
    async fn build_modals(&self, http: &Client, cache: &InMemoryCache, _: A) -> Result<Vec<R>>;
}

/// A value that can build buttons using the provided arguments.
pub trait BuildButtons<A = (), R = Button>
where
    R: Into<Button>,
{
    /// Builds buttons using the provided arguments.
    fn build_buttons(&self, disabled: bool, _: A) -> Result<Vec<R>>;
}

/// A value that can build buttons using the provided arguments.
#[async_trait::async_trait]
pub trait BuildButtonsAsync<A = (), R = Button>
where
    Self: Sync,
    A: Send + Sync,
    R: Into<Button> + Send,
{
    /// Builds buttons using the provided arguments.
    async fn build_buttons(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        disabled: bool,
        _: A,
    ) -> Result<Vec<R>>;
}

/// A value that can build an text input using the provided arguments.
pub trait BuildTextInputs<A = (), R = TextInput>
where
    R: Into<TextInput>,
{
    /// Builds text inputs using the provided arguments.
    fn build_text_inputs(&self, _: A) -> Result<Vec<R>>;
}

/// A value that can build texts input using the provided arguments.
#[async_trait::async_trait]
pub trait BuildTextInputsAsync<A = (), R = TextInput>
where
    Self: Sync,
    A: Send + Sync,
    R: Into<TextInput> + Send,
{
    /// Builds text inputs using the provided arguments.
    async fn build_text_inputs(&self, http: &Client, cache: &InMemoryCache, _: A)
    -> Result<Vec<R>>;
}

/// A value that can build select menus using the provided arguments.
pub trait BuildSelectMenus<A = (), R = SelectMenu>
where
    R: Into<SelectMenu>,
{
    /// Builds select menus using the provided arguments.
    fn build_select_menus(&self, _: A) -> Result<Vec<R>>;
}

/// A value that can build select menus using the provided arguments.
#[async_trait::async_trait]
pub trait BuildSelectMenusAsync<A = (), R = SelectMenu>
where
    Self: Sync,
    A: Send + Sync,
    R: Into<SelectMenu> + Send,
{
    /// Builds select menus using the provided arguments.
    async fn build_select_menus(
        &self,
        http: &Client,
        cache: &InMemoryCache,
        _: A,
    ) -> Result<Vec<R>>;
}

/// Builds buttons using the provided arguments and automatically puts them
/// into action rows.
pub fn button_rows(buttons: impl IntoIterator<Item = impl Into<Button>>) -> Vec<Component> {
    let mut components = Vec::with_capacity(5);
    let mut action_row = Vec::with_capacity(5);

    for button in buttons {
        if action_row.len() < 5 {
            action_row.push(Component::Button(button.into()));
        } else {
            components.push(ActionRowBuilder::new(action_row).into());
            action_row = Vec::with_capacity(5);
        }
    }

    if !action_row.is_empty() {
        components.push(ActionRowBuilder::new(action_row).into());
    }

    components
}
