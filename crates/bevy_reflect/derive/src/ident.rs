use proc_macro2::Ident;

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
