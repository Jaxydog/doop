use crate::prelude::*;

macro_rules! define_traits {
    ($try_as:ident { $ta_fn:ident }, $as:ident { $as_fn:ident }, $async_as:ident { $ay_fn:ident } => $output:ty) => {
        pub trait $try_as {
            type Args;

            fn $ta_fn(&self, _: Self::Args) -> Result<$output>;
        }
        pub trait $as {
            type Args;

            fn $as_fn(&self, _: Self::Args) -> $output;
        }
        #[async_trait]
        pub trait $async_as {
            type Args;

            async fn $ay_fn(&self, http: &impl CacheHttp, _: Self::Args) -> Result<$output>;
        }

        impl<T: $as> $try_as for T {
            type Args = T::Args;

            fn $ta_fn(&self, args: Self::Args) -> Result<$output> {
                Ok(self.$as_fn(args))
            }
        }
    };
    (disableable $try_as:ident { $ta_fn:ident }, $as:ident { $as_fn:ident }, $async_as:ident { $ay_fn:ident } => $output:ty) => {
        pub trait $try_as {
            type Args;

            fn $ta_fn(&self, disable: bool, _: Self::Args) -> Result<$output>;
        }
        pub trait $as {
            type Args;

            fn $as_fn(&self, disable: bool, _: Self::Args) -> $output;
        }
        #[async_trait]
        pub trait $async_as {
            type Args;

            async fn $ay_fn(
                &self,
                http: &impl CacheHttp,
                disable: bool,
                _: Self::Args,
            ) -> Result<$output>;
        }

        impl<T: $as> $try_as for T {
            type Args = T::Args;

            fn $ta_fn(&self, disable: bool, args: Self::Args) -> Result<$output> {
                Ok(self.$as_fn(disable, args))
            }
        }
    };
}

define_traits!(disableable TryAsButton { try_as_button }, AsButton { as_button }, AsButtonAsync { as_button } => CreateButton);
define_traits!(disableable TryAsButtons { try_as_buttons }, AsButtons { as_buttons }, AsButtonsAsync { as_buttons } => Vec<CreateButton>);
define_traits!(TryAsEmbed { try_as_embed }, AsEmbed { as_embed }, AsEmbedAsync { as_embed } => CreateEmbed);
define_traits!(TryAsInputText { try_as_input_text }, AsInputText { as_input_text }, AsInputTextAsync { as_input_text } => CreateInputText);
define_traits!(TryAsMessage { try_as_message }, AsMessage { as_message }, AsMessageAsync { as_message } => CreateMessage);
define_traits!(TryAsModal { try_as_modal }, AsModal { as_modal }, AsModalAsync { as_modal } => CreateModal);
