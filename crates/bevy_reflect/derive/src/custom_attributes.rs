use crate::utility::SpannedString;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, Lit, LitBool, LitStr, Path, Token};

#[derive(Default, Clone)]
pub(crate) struct CustomAttributes {
    attributes: HashMap<SpannedString, Expr>,
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
    pub fn insert(&mut self, name: impl Into<SpannedString>, value: Expr) -> syn::Result<()> {
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
    /// - `#[reflect(@hidden))]`
    /// - `#[reflect(@foo = Bar::baz("qux"))]`
    /// - `#[reflect(@min = 0.0, @max = 1.0)]`
    pub fn parse_custom_attribute(&mut self, input: ParseStream) -> syn::Result<()> {
        input.parse::<Token![@]>()?;

        let path = Path::parse_mod_style(input)?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse::<Expr>()?
        } else {
            Expr::Lit(ExprLit {
                attrs: Vec::new(),
                lit: Lit::Bool(LitBool::new(true, path.span())),
            })
        };

        let mut name = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        if path.leading_colon.is_some() {
            name.insert_str(0, "::");
        }

        self.insert(
            // Note that the call to `.span()` will only return the span of the first token.
            // This isn't ideal, but should be fine— especially for single-ident names.
            // See: https://docs.rs/syn/2.0.48/syn/spanned/index.html#limitations
            LitStr::new(&name, path.span()),
            value,
        )?;

        Ok(())
    }
}
