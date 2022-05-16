//! General-purpose utility functions for internal usage within this crate.

use bevy_macro_utils::BevyManifest;
use proc_macro2::{Ident, Span};
use quote::quote;
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

/// Returns a token stream of comma-separated underscores for the given count.
///
/// This is useful for creating tuple field accessors:
///
/// ```ignore
/// let empties = underscores(2);
/// quote! {
///   let (#empties value, ..) = (10, 20, 30);
///   assert_eq!(30, value);
/// }
/// ```
///
/// > Note: This automatically handles the trailing comma.
///
pub(crate) fn underscores(count: usize) -> proc_macro2::TokenStream {
    let mut output = proc_macro2::TokenStream::new();
    for _ in 0..count {
        output = quote! {
            #output _,
        }
    }
    output
}

/// Helper struct used to process an iterator of `Result<Vec<T>, syn::Error>`,
/// combining errors into one along the way.
pub(crate) struct ResultSifter<T> {
    items: Vec<T>,
    errors: Option<syn::Error>,
}

impl<T> Default for ResultSifter<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            errors: None,
        }
    }
}

impl<T> ResultSifter<T> {
    /// Sift the given result, combining errors if necessary.
    pub fn sift(&mut self, result: Result<T, syn::Error>) {
        match result {
            Ok(data) => self.items.push(data),
            Err(err) => {
                if let Some(ref mut errors) = self.errors {
                    errors.combine(err);
                } else {
                    self.errors = Some(err);
                }
            }
        }
    }

    /// Associated method that provides a convenient implementation for [`Iterator::fold`].
    ///
    /// [`Iterator::fold`]: core::iter::traits::iterator::Iterator::fold
    pub fn fold(mut sifter: Self, result: Result<T, syn::Error>) -> Self {
        sifter.sift(result);
        sifter
    }

    /// Complete the sifting process and return the final result.
    pub fn finish(self) -> Result<Vec<T>, syn::Error> {
        if let Some(errors) = self.errors {
            Err(errors)
        } else {
            Ok(self.items)
        }
    }
}
