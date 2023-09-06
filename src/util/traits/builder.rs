use twilight_model::channel::message::component::{Button, SelectMenu};
use twilight_model::channel::message::Embed;

use crate::util::Modal;

/// Defines builder traits.
///
/// # Examples
///
/// ```
/// traits! {
///     [BuildEmbed, BuildEmbeds, BuildEmbedAsync, BuildEmbedsAsync] = { build_embed, build_embeds } -> Embed;
/// }
/// ```
macro_rules! traits {
    ($(
        [$one_trait:ident, $many_trait:ident, $one_trait_async:ident, $many_trait_async:ident] = { $one_fn:ident, $many_fn:ident } -> $ret:ty;)*
    ) => {$(
        /// Builds a value.
        pub trait $one_trait {
            /// The arguments provided to the builder method.
            type Arguments;
            /// The type returned if building failed.
            type Error;

            /// Builds a value from the given arguments.
            ///
            /// # Errors
            ///
            /// This function will return an error if building fails.
            fn $one_fn(&self, _: Self::Arguments) -> Result<$ret, Self::Error>;
        }

        /// Builds a list of values.
        pub trait $many_trait {
            /// The arguments provided to the builder method.
            type Arguments;
            /// The type returned if building failed.
            type Error;

            /// Builds values from the given arguments.
            ///
            /// # Errors
            ///
            /// This function will return an error if building fails.
            fn $many_fn(&self, _: Self::Arguments) -> Result<Vec<$ret>, Self::Error>;
        }

        /// Builds a value.
        #[::async_trait::async_trait]
        pub trait $one_trait_async {
            /// The arguments provided to the builder method.
            type Arguments;
            /// The type returned if building failed.
            type Error;

            /// Builds a value from the given arguments.
            ///
            /// # Errors
            ///
            /// This function will return an error if building fails.
            async fn $one_fn(&self, api: $crate::bot::interact::Api<'_>, _: Self::Arguments) -> Result<$ret, Self::Error>;
        }

        /// Builds a list of values.
        #[::async_trait::async_trait]
        pub trait $many_trait_async {
            /// The arguments provided to the builder method.
            type Arguments;
            /// The type returned if building failed.
            type Error;

            /// Builds values from the given arguments.
            ///
            /// # Errors
            ///
            /// This function will return an error if building fails.
            async fn $many_fn(&self, api: $crate::bot::interact::Api<'_>, _: Self::Arguments) -> Result<Vec<$ret>, Self::Error>;
        }
    )*};
}

traits! {
    [BuildButton, BuildButtons, BuildButtonAsync, BuildButtonsAsync] = { build_button, build_buttons } -> Button;
    [BuildEmbed, BuildEmbeds, BuildEmbedAsync, BuildEmbedsAsync] = { build_embed, build_embeds } -> Embed;
    [BuildModal, BuildModals, BuildModalAsync, BuildModalsAsync] = { build_modal, build_modals } -> Modal;
    [BuildSelectMenu, BuildSelectMenus, BuildSelectMenuAsync, BuildSelectMenusAsync] = { build_select_menu, build_select_menus } -> SelectMenu;
}
