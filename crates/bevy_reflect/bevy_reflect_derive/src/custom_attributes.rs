use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parenthesized, Lit, LitStr, Path, Token};

#[derive(Default, Clone)]
pub(crate) struct CustomAttributes {
    attributes: HashMap<String, Lit>,
}

impl CustomAttributes {
    /// Generates a `TokenStream` for `CustomAttributes` construction.
    pub fn to_tokens(&self, bevy_reflect_path: &Path) -> TokenStream {
        let attributes = self.attributes.iter().map(|(name, value)| {
            quote! {
                .with_attribute(#name, #value)
            }
        });

        quote! {
            #bevy_reflect_path::attributes::CustomAttributes::default()
                #(#attributes)*
        }
    }

    /// Inserts a custom attribute into the map.
    pub fn insert(&mut self, name: LitStr, value: Lit) -> syn::Result<()> {
        let key = name.value();
        if self.attributes.contains_key(&key) {
            return Err(syn::Error::new_spanned(name, "duplicate custom attribute"));
        }

        self.attributes.insert(key, value);

        Ok(())
    }

    /// Parse `@` (custom attribute) attribute.
    ///
    /// Examples:
    /// - `#[reflect(@(foo = "bar"))]`
    /// - `#[reflect(@(min = 0.0, max = 1.0))]`
    pub fn parse_custom_attribute(&mut self, input: ParseStream) -> syn::Result<()> {
        input.parse::<Token![@]>()?;

        let content;
        parenthesized!(content in input);

        let custom_attrs = content.parse_terminated(CustomAttribute::parse, Token![,])?;

        for custom_attr in custom_attrs {
            let name = custom_attr
                .name
                .iter()
                .map(Ident::to_string)
                .collect::<Vec<_>>()
                .join("::");

            self.insert(
                LitStr::new(&name, custom_attr.name.span()),
                custom_attr.value,
            )?;
        }

        Ok(())
    }
}

pub(crate) struct CustomAttribute {
    name: Punctuated<Ident, Token![::]>,
    _eq: Token![=],
    value: Lit,
}

impl Parse for CustomAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: Punctuated::<Ident, Token![::]>::parse_separated_nonempty(input)?,
            _eq: input.parse::<Token![=]>()?,
            value: input.parse::<Lit>()?,
        })
    }
}
