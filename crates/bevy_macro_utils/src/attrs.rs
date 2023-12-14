use syn::{Expr, ExprLit, Lit};

use crate::symbol::Symbol;

/// Get a [literal string](struct@syn::LitStr) from the provided [expression](Expr).
pub fn get_lit_str(attr_name: Symbol, value: &Expr) -> syn::Result<&syn::LitStr> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = &value
    {
        Ok(lit)
    } else {
        Err(syn::Error::new_spanned(
            value,
            format!("expected {attr_name} attribute to be a string: `{attr_name} = \"...\"`"),
        ))
    }
}

/// Get a [literal boolean](struct@syn::LitBool) from the provided [expression](Expr) as a [`bool`].
pub fn get_lit_bool(attr_name: Symbol, value: &Expr) -> syn::Result<bool> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Bool(lit),
        ..
    }) = &value
    {
        Ok(lit.value())
    } else {
        Err(syn::Error::new_spanned(
            value,
            format!("expected {attr_name} attribute to be a bool value, `true` or `false`: `{attr_name} = ...`"),
        ))?
    }
}
