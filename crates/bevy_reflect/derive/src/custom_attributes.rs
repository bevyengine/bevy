use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::ParseStream;
use syn::{Expr, Path, Token};

#[derive(Default, Clone)]
pub(crate) struct CustomAttributes {
    attributes: Vec<Expr>,
}

impl CustomAttributes {
    /// Generates a `TokenStream` for `CustomAttributes` construction.
    pub fn to_tokens(&self, bevy_reflect_path: &Path) -> TokenStream {
        let attributes = self.attributes.iter().map(|value| {
            quote! {
                .with_attribute(#value)
            }
        });

        quote! {
            #bevy_reflect_path::attributes::CustomAttributes::default()
                #(#attributes)*
        }
    }

    /// Inserts a custom attribute into the list.
    pub fn push(&mut self, value: Expr) -> syn::Result<()> {
        self.attributes.push(value);
        Ok(())
    }

    /// Parse `@` (custom attribute) attribute.
    ///
    /// Examples:
    /// - `#[reflect(@Foo))]`
    /// - `#[reflect(@Bar::baz("qux"))]`
    /// - `#[reflect(@0..256u8)]`
    pub fn parse_custom_attribute(&mut self, input: ParseStream) -> syn::Result<()> {
        input.parse::<Token![@]>()?;
        self.push(input.parse()?)
    }
}
