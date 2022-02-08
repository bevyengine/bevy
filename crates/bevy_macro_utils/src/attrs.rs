use syn::DeriveInput;

use crate::Symbol;

pub fn get_attr_meta_items(
    attr: &syn::Attribute,
    attr_name: &'static str,
) -> syn::Result<Vec<syn::NestedMeta>> {
    if !attr.path.is_ident(attr_name) {
        return Ok(Vec::new());
    }

    match attr.parse_meta()? {
        syn::Meta::List(meta) => Ok(meta.nested.into_iter().collect()),
        other => Err(syn::Error::new_spanned(
            other,
            format!("expected #[{}(...)]", attr_name),
        )),
    }
}

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
