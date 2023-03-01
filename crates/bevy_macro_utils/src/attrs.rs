use syn::DeriveInput;

use crate::symbol::Symbol;

pub fn parse_attrs(ast: &DeriveInput, attr_name: Symbol) -> syn::Result<Vec<syn::ExprAssign>> {
    let mut list = Vec::new();
    for attr in ast.attrs.iter().filter(|a| a.path == attr_name) {
        let args = attr.parse_args_with(|parse: &syn::parse::ParseBuffer| {
            parse.parse_terminated::<syn::ExprAssign, syn::token::Comma>(|parse| parse.parse())
        })?;
        list.extend(args);
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
