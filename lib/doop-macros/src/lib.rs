//! Provides procedural macro definitions for the Doop Discord bot.
#![deny(clippy::expect_used, unsafe_code, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::todo, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions)]

use proc_macro::TokenStream;

mod storage;

/// Derives the [`Storage`](<doop_storage::Storage>) trait for the deriving type.
///
/// # Examples
///
/// ```
/// #[derive(Storage, Serialize, Deserialize)]
/// #[format(Compress<MsgPack, 3>)]
/// #[path("{}/{}", u64, String)]
/// struct Data {
///     id: Id<GuildMarker>,
///     time: OffsetDateTime,
/// }
/// ```
#[inline]
#[proc_macro_derive(Storage, attributes(format, location))]
pub fn storage(input: TokenStream) -> TokenStream {
    crate::storage::procedure(input)
}
