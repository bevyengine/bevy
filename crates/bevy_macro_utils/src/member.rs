use syn::{Ident, Member};

/// Converts an optional identifier or index into a `Member` variant.
///
/// This is useful for when u want to acces a field inside a `quote!` block regardless of whether it is an identifier or an index.
///
/// # Example
/// ```rust
/// use syn::{Ident, parse_str};
/// use quote::quote;
/// use bevy_macro_utils::as_member;
///
/// let ident = Some(parse_str::<Ident>("my_field").unwrap());
/// let index = 0;
/// let member = as_member(ident, index);
/// quote! {
///    self.#member.do_something();
/// };
/// ```
///
pub fn as_member(ident: &Option<Ident>, index: usize) -> Member {
    ident.clone().map_or(Member::from(index), Member::Named)
}
