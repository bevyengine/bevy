//! General-purpose utility functions for internal usage within this crate.

use bevy_macro_utils::BevyManifest;
use proc_macro2::{Ident, Span};
use syn::Path;

/// Returns the correct path for `bevy_reflect`.
pub(crate) fn get_bevy_reflect_path() -> Path {
    BevyManifest::get_path_direct("bevy_reflect")
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
