use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::ParseStream;
use syn::{parse_macro_input, Attribute, DeriveInput, Error, LitStr, Result, Token, Type};

struct StorageAttribute {
    format: Type,
    at: (LitStr, Vec<Type>),
}

fn parse_storage_attribute(attribute: &Attribute) -> Result<StorageAttribute> {
    syn::custom_keyword!(format);
    syn::custom_keyword!(at);

    attribute.parse_args_with(|input: ParseStream| {
        input.parse::<format>()?;
        input.parse::<Token!(=)>()?;

        let format = input.parse()?;

        input.parse::<Token!(,)>()?;
        input.parse::<at>()?;
        input.parse::<Token!(=)>()?;

        let at = {
            let fmt = input.parse()?;
            let mut args = vec![];

            while !input.is_empty() && input.peek(Token!(,)) {
                input.parse::<Token!(,)>()?;

                if !input.is_empty() {
                    args.push(input.parse()?);
                }
            }

            (fmt, args)
        };

        Ok(StorageAttribute { format, at })
    })
}

#[allow(clippy::unwrap_used)]
pub fn procedure(input: TokenStream) -> TokenStream {
    let DeriveInput { attrs, ident, generics, .. } = parse_macro_input!(input as DeriveInput);
    let Some(attribute) = attrs.iter().find(|a| a.path().is_ident("storage")) else {
        return Error::new(ident.span(), "the `storage` attribute must be configured")
            .into_compile_error()
            .into();
    };
    let StorageAttribute { format, at: (fmt, args) } = match parse_storage_attribute(attribute) {
        Ok(value) => value,
        Err(error) => return error.to_compile_error().into(),
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fmt_args = (0..args.len()).map(|n| format_ident!("_{n}")).collect::<Vec<_>>();

    quote! {
        impl #impl_generics ::doop_storage::Storage<
            ::doop_storage::FileSystem,
            ::doop_storage::FileKey<Self, #format>,
            ::doop_storage::FileVal<Self, #format>
        > for #ident #ty_generics #where_clause
        {
            type Arguments = (#(#args),*);
            type Format = #format;

            fn stored((#(#fmt_args),*): Self::Arguments) -> ::doop_storage::FileKey<Self, #format> {
                format!(#fmt, #(#fmt_args),*).into()
            }
        }
    }
    .into()
}
