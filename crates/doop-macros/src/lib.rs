#![doc = env!("CARGO_PKG_DESCRIPTION")]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![warn(clippy::nursery, clippy::pedantic, clippy::todo, missing_docs)]

use proc_macro::TokenStream;

mod global;
mod storage;

/// Creates a once-locked static variable with an associated getter function.
///
/// # Examples
///
/// ```
/// global! {
///     /// A global string value.
///     static STRING: String = String::from("this is a test string!");
/// }
/// ```
#[inline]
#[proc_macro]
pub fn global(input: TokenStream) -> TokenStream { crate::global::procedure(input) }

/// Derives the [`Storage`](<doop_storage::Storage>) trait for the type.
///
/// # Examples
///
/// ```
/// #[derive(Storage, Serialize, Deserialize)]
/// #[storage(format = Compress<MsgPack, 3>, at = "{}/{}", u64, String)]
/// struct Stored<T: Serialize + for<'de> Deserialize<'de>> {
///     id: u64,
///     data: T,
/// }
/// ```
#[inline]
#[proc_macro_derive(Storage, attributes(storage))]
pub fn storage(input: TokenStream) -> TokenStream { crate::storage::procedure(input) }
