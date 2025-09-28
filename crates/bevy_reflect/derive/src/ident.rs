use proc_macro2::{Ident, Span};

/// Returns the "reflected" ident for a given string.
///
/// # Example
///
/// ```
/// # use proc_macro2::Ident;
/// # // We can't import this method because of its visibility.
/// # fn get_reflect_ident(name: &str) -> Ident {
/// #     let reflected = format!("Reflect{name}");
/// #     Ident::new(&reflected, proc_macro2::Span::call_site())
/// # }
/// let reflected: Ident = get_reflect_ident("Hash");
/// assert_eq!("ReflectHash", reflected.to_string());
/// ```
pub(crate) fn get_reflect_ident(name: &str) -> Ident {
    let reflected = format!("Reflect{name}");
    Ident::new(&reflected, Span::call_site())
}
