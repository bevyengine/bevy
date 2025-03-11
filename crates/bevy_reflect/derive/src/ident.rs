use proc_macro2::Ident;
use syn::Member;

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

/// Returns a [`Member`] made of `ident` or `index` if `ident` is `None`.
///
/// Rust struct syntax allows for `Struct { foo: "string" }` with explicitly
/// named fields. It allows the `Struct { 0: "string" }` syntax when the struct
/// is declared as a tuple struct.
///
/// ```
/// struct Foo { field: &'static str }
/// struct Bar(&'static str);
/// let Foo { field } = Foo { field: "hi" };
/// let Bar { 0: field } = Bar { 0: "hello" };
/// let Bar(field) = Bar("hello"); // more common syntax
/// ```
///
/// This function helps field access in contexts where you are declaring either
/// a tuple struct or a struct with named fields. If you don't have a field name,
/// it means that you must access the field through an index.
pub(crate) fn ident_or_index(ident: Option<&Ident>, index: usize) -> Member {
    ident.map_or_else(
        || Member::Unnamed(index.into()),
        |ident| Member::Named(ident.clone()),
    )
}
