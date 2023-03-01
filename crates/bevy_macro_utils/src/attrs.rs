use syn::DeriveInput;

use crate::symbol::Symbol;

pub struct NamedArg {
    pub path: syn::Path,
    pub expr: syn::Expr,
}

pub fn parse_attrs(ast: &DeriveInput, attr_name: Symbol) -> syn::Result<Vec<NamedArg>> {
    let mut list = Vec::new();
    for attr in ast.attrs.iter().filter(|a| a.path == attr_name) {
        let args = attr.parse_args_with(|parse: &syn::parse::ParseBuffer| {
            parse.parse_terminated::<syn::ExprAssign, syn::token::Comma>(|parse| parse.parse())
        })?;
        for syn::ExprAssign { left, right, .. } in args {
            let path = match *left {
                syn::Expr::Path(path) => path.path,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "invalid attribute: expected a path identifier",
                    ))
                }
            };
            list.push(NamedArg { path, expr: *right })
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
