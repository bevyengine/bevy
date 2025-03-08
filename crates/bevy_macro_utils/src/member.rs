use syn::{Ident, Member};

/// Converts an optional identifier or index into a [`syn::Member`] variant.
///
/// This is useful for when u want to acces a field inside a `quote!` block regardless of whether it is an identifier or an index.
/// There is also [`syn::Field::members`], but when u are working with single fields this method won't work.
///
/// Rust struct syntax allows for `Struct { foo: "string" }` with explicitly
/// named fields. It allows the `Struct { 0: "string" }` syntax when the struct
/// is declared as a tuple struct.
/// # Example
/// ```rust
/// use syn::{Ident, parse_str, DeriveInput, Data, DataStruct};
/// use quote::quote;
/// use bevy_macro_utils::as_member;
///
/// let ast: DeriveInput = syn::parse_str(
///     r#"
///     struct Mystruct {
///         field: usize,
///         #[my_derive]
///         other_field: usize
///     }
/// "#,
/// )
/// .unwrap();
///
/// let Data::Struct(DataStruct { fields, .. }) = &ast.data else { return };
///
/// let field_members = fields
///     .iter()
///     .enumerate()
///     .filter(|(_, field)| field.attrs.iter().any(|attr| attr.path().is_ident("my_derive")))
///     .map(|(i, field)| { as_member(&field.ident, i) });
///
/// // it won't matter now if it's a named field or a unnamed field. e.g self.field or self.0
/// quote!(
///     #(self.#field_members.do_something();)*
///     );
///
/// ```
///
pub fn as_member(ident: &Option<Ident>, index: usize) -> Member {
    ident.clone().map_or(Member::from(index), Member::Named)
}
