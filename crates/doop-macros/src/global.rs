use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Expr, Ident, Result, Token, Type};

struct Global {
    attributes: Vec<Attribute>,
    name: Ident,
    stored: Type,
    initializer: Expr,
}

impl Parse for Global {
    fn parse(input: ParseStream) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;

        input.parse::<Token!(static)>()?;

        let name = input.parse()?;

        input.parse::<Token!(:)>()?;

        let stored = input.parse()?;

        input.parse::<Token!(=)>()?;

        let initializer = input.parse()?;

        input.parse::<Token!(;)>()?;

        Ok(Self { attributes, name, stored, initializer })
    }
}

#[repr(transparent)]
struct GlobalList(Vec<Global>);

impl Parse for GlobalList {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut globals = vec![];

        while !input.is_empty() {
            globals.push(input.parse()?);
        }

        Ok(Self(globals))
    }
}

pub fn procedure(input: TokenStream) -> TokenStream {
    let GlobalList(globals) = parse_macro_input!(input);
    let mut stream = TokenStream::new();

    for Global { attributes, name, stored, initializer } in globals {
        let getter = Ident::new(name.to_string().to_lowercase().as_str(), Span::call_site());

        let sync_assertion = quote_spanned! {
            stored.span() => struct _AssertSync where #stored: ::std::marker::Sync;
        };
        let sized_assertion = quote_spanned! {
            stored.span() => struct _AssertSized where #stored: ::std::marker::Sized;
        };

        stream.extend(TokenStream::from(quote! {
            static #name: ::std::sync::OnceLock<#stored> = ::std::sync::OnceLock::new();

            #(#attributes)*
            #[inline] pub fn #getter() -> &'static #stored {
                #sync_assertion
                #sized_assertion

                #name.get_or_init(|| #initializer)
            }
        }));
    }

    stream
}
