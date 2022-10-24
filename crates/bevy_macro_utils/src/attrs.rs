use syn::DeriveInput;

use crate::symbol::Symbol;

pub fn parse_attrs(ast: &DeriveInput, attr_name: Symbol) -> syn::Result<Vec<syn::NestedMeta>> {
    let mut list = Vec::new();
    for attr in ast.attrs.iter().filter(|a| a.path == attr_name) {
        match attr.parse_meta()? {
            syn::Meta::List(meta) => list.extend(meta.nested.into_iter()),
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    format!("expected #[{}(...)]", attr_name),
                ))
            }
        }
    }
    Ok(list)
}

pub fn get_lit_str(attr_name: Symbol, lit: &syn::Lit) -> syn::Result<&syn::LitStr> {
    if let syn::Lit::Str(lit) = lit {
        Ok(lit)
    } else {
        Err(syn::Error::new_spanned(
            lit,
            format!(
                "expected {} attribute to be a string: `{} = \"...\"`",
                attr_name, attr_name
            ),
        ))
    }
}

pub fn get_lit_bool(attr_name: Symbol, lit: &syn::Lit) -> syn::Result<bool> {
    if let syn::Lit::Bool(lit) = lit {
        Ok(lit.value())
    } else {
        Err(syn::Error::new_spanned(
            lit,
            format!(
                "expected {} attribute to be a bool value, `true` or `false`: `{} = ...`",
                attr_name, attr_name
            ),
        ))
    }
}
