use bevy_macro_utils::BevyManifest;
use proc_macro2::{Ident, Span};
use syn::Path;

pub(crate) fn get_bevy_reflect_path() -> Path {
    BevyManifest::default().get_path("bevy_reflect")
}

/// Returns the "reflected" ident for a given string.
///
/// # Example
///
/// ```ignore
/// let reflected: Ident = get_reflect_ident("Hash");
/// assert_eq!("ReflectHash", reflected.to_string());
/// ```
pub(crate) fn get_reflect_ident(name: &str) -> Ident {
    let reflected = format!("Reflect{}", name);
    Ident::new(&reflected, Span::call_site())
}
