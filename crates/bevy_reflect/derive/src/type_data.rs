use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, token, Expr, Token};

/// A `TypeData` registration.
///
/// This would be the `Default` and `Hash(custom_hash_fn)` in
/// `#[reflect(Default, Hash(custom_hash_fn))]`.
#[derive(Clone)]
pub(crate) struct TypeDataRegistration {
    ident: Ident,
    reflect_ident: Ident,
    args: Punctuated<Expr, Token![,]>,
}

impl TypeDataRegistration {
    /// The shortened ident of the registration.
    ///
    /// This would be `Default` in `#[reflect(Default)]`.
    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    /// The full reflection ident of the registration.
    ///
    /// This would be `ReflectDefault` in `#[reflect(Default)]`.
    pub fn reflect_ident(&self) -> &Ident {
        &self.reflect_ident
    }

    /// The optional arguments of the type data.
    pub fn args(&self) -> &Punctuated<Expr, Token![,]> {
        &self.args
    }
}

impl Parse for TypeDataRegistration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        let reflect_ident = crate::ident::get_reflect_ident(&ident);

        let args = if input.peek(token::Paren) {
            let content;
            parenthesized!(content in input);
            content.parse_terminated(Expr::parse, Token![,])?
        } else {
            Default::default()
        };

        Ok(Self {
            ident,
            reflect_ident,
            args,
        })
    }
}
