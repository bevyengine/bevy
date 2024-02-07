use crate::utility::SpannedString;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parenthesized, Lit, LitStr, Path, Token};

#[derive(Default, Clone)]
pub(crate) struct CustomAttributes {
    attributes: HashMap<SpannedString, Lit>,
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
    pub fn insert(&mut self, name: impl Into<SpannedString>, value: Lit) -> syn::Result<()> {
        let name = name.into();
        if self.attributes.contains_key(&name) {
            return Err(syn::Error::new_spanned(name, "duplicate custom attribute"));
        }

        self.attributes.insert(name, value);

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
            let mut name = custom_attr
                .name
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if custom_attr.name.leading_colon.is_some() {
                name.insert_str(0, "::");
            }

            self.insert(
                // Note that the call to `.span()` will only return the span of the first token.
                // This isn't ideal, but should be fine— especially for single-ident names.
                // See: https://docs.rs/syn/2.0.48/syn/spanned/index.html#limitations
                LitStr::new(&name, custom_attr.name.span()),
                custom_attr.value,
            )?;
        }

        Ok(())
    }
}

pub(crate) struct CustomAttribute {
    name: Path,
    _eq: Token![=],
    value: Lit,
}

impl Parse for CustomAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: Path::parse_mod_style(input)?,
            _eq: input.parse::<Token![=]>()?,
            value: input.parse::<Lit>()?,
        })
    }
}
