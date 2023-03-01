use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::DeriveInput;

use crate::symbol::Symbol;

pub fn parse_attrs(ast: &DeriveInput, attr_name: Symbol) -> syn::Result<Vec<TokenStream2>> {
    let mut list = Vec::new();
    for attr in ast.attrs.iter().filter(|a| a.path == attr_name) {
        match attr.parse_meta() {
            Ok(syn::Meta::List(meta)) => {
                list.extend(meta.nested.into_iter().map(ToTokens::into_token_stream))
            }
            _ => list.push(attr.tokens.clone()),
        }
    }
    Ok(list)
}

pub fn get_lit_str(attr_name: Symbol, expr: &syn::Expr) -> syn::Result<&syn::LitStr> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = expr
    {
        Ok(lit)
    } else {
        Err(syn::Error::new_spanned(
            expr,
            format!("expected {attr_name} attribute to be a string: `{attr_name} = \"...\"`"),
        ))
    }
}

pub fn get_lit_bool(attr_name: Symbol, lit: &syn::Lit) -> syn::Result<bool> {
    if let syn::Lit::Bool(lit) = lit {
        Ok(lit.value())
    } else {
        Err(syn::Error::new_spanned(
            lit,
            format!("expected {attr_name} attribute to be a bool value, `true` or `false`: `{attr_name} = ...`"),
        ))
    }
}
