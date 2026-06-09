use proc_macro2::Ident;
use syn::Path;

/// Returns the "reflected" ident for a given string.
///
/// # Example
///
/// ```
/// # use proc_macro2::{Ident, Span};
/// # // We can't import this method because of its visibility.
/// # fn get_reflect_ident(base_ident: &Ident) -> Ident {
/// #     let reflected = format!("Reflect{base_ident}");
/// #     Ident::new(&reflected, base_ident.span())
/// # }
/// let reflected: Ident = get_reflect_ident(&Ident::new("Hash", Span::call_site()));
/// assert_eq!("ReflectHash", reflected.to_string());
/// ```
pub(crate) fn get_reflect_ident(base_ident: &Ident) -> Ident {
    let reflected = format!("Reflect{base_ident}");
    Ident::new(&reflected, base_ident.span())
}

pub(crate) fn get_reflect_path(path: &Path) -> Path {
    let mut path = path.clone();

    match path.segments.last_mut() {
        Some(segment) if !is_reflected_ident(&segment.ident) => {
            segment.ident = get_reflect_ident(&segment.ident);
        }
        _ => {}
    }
    path
}

fn is_reflected_ident(ident: &Ident) -> bool {
    let ident_str = ident.to_string();
    ident_str.starts_with("Reflect")
}
