use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::ParseStream;
use syn::{parse_macro_input, Attribute, DeriveInput, Error, LitStr, Result, Token, Type};

struct FormatAttribute(Type);

fn parse_format_attribute(attribute: &Attribute) -> Result<FormatAttribute> {
    attribute.parse_args_with(|input: ParseStream| Ok(FormatAttribute(input.parse()?)))
}

struct LocationAttribute(LitStr, Vec<Type>);

fn parse_location_attribute(attribute: &Attribute) -> Result<LocationAttribute> {
    attribute.parse_args_with(|input: ParseStream| {
        let literal = input.parse::<LitStr>()?;
        let mut arguments = vec![];

        while !input.is_empty() && input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            if !input.is_empty() {
                arguments.push(input.parse()?);
            }
        }

        Ok(LocationAttribute(literal, arguments))
    })
}

pub fn procedure(input: TokenStream) -> TokenStream {
    let DeriveInput { attrs, ident, generics, .. } = parse_macro_input!(input as DeriveInput);

    let Some(attribute) = attrs.iter().find(|a| a.path().is_ident("format")) else {
        return Error::new(ident.span(), "the `format` attribute must be configured")
            .into_compile_error()
            .into();
    };
    let FormatAttribute(format) = match self::parse_format_attribute(attribute) {
        Ok(value) => value,
        Err(error) => return error.into_compile_error().into(),
    };

    let Some(attribute) = attrs.iter().find(|a| a.path().is_ident("location")) else {
        return Error::new(ident.span(), "the `location` attribute must be configured")
            .into_compile_error()
            .into();
    };
    let LocationAttribute(location, args) = match self::parse_location_attribute(attribute) {
        Ok(value) => value,
        Err(error) => return error.into_compile_error().into(),
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fmt_args = (0 .. args.len()).map(|n| format_ident!("_{n}")).collect::<Vec<_>>();

    quote! {
        impl #impl_generics ::doop_storage::Stored for #ident #ty_generics #where_clause {
            type Arguments = (#(#args),*);
            type Format = #format;

            fn stored((#(#fmt_args),*): Self::Arguments)-> ::doop_storage::Key<Self, Self::Format> {
                format!(#location, #(#fmt_args),*).into()
            }
        }
    }
    .into()
}
