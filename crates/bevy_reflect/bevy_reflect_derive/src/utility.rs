//! General-purpose utility functions for internal usage within this crate.

use crate::field_attributes::ReflectIgnoreBehavior;
use bevy_macro_utils::BevyManifest;
use bit_set::BitSet;
use proc_macro2::{Ident, Span};
use syn::{Member, Path};

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

/// Helper struct used to process an iterator of `Result<Vec<T>, syn::Error>`,
/// combining errors into one along the way.
pub(crate) struct ResultSifter<T> {
    items: Vec<T>,
    errors: Option<syn::Error>,
}

/// Returns a `Member` made of `ident` or `index` if `ident` is None.
///
/// Rust struct syntax allows for `Struct { foo: "string" }` with explicitly
/// named fields. It allows the `Struct { 0: "string" }` syntax when the struct
/// is declared as a tuple struct.
///
/// ```
/// # fn main() {
/// struct Foo { field: &'static str }
/// struct Bar(&'static str);
/// let Foo { field } = Foo { field: "hi" };
/// let Bar { 0: field } = Bar { 0: "hello" };
/// let Bar(field) = Bar("hello"); // more common syntax
/// # }
/// ```
///
/// This function helps field access in context where you are declaring either
/// a tuple struct or a struct with named fields. If you don't have a field name,
/// it means you need to access the struct through an index.
pub(crate) fn ident_or_index(ident: Option<&Ident>, index: usize) -> Member {
    // TODO(Quality) when #4761 is merged, code that does this should be replaced
    // by a call to `ident_or_index`.
    ident.map_or_else(
        || Member::Unnamed(index.into()),
        |ident| Member::Named(ident.clone()),
    )
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

/// Converts an iterator over ignore behaviour of members to a bitset of ignored members.
///
/// Takes into account the fact that always ignored (non-reflected) members are skipped.
///
/// # Example
/// ```rust,ignore
/// pub struct HelloWorld {
///     reflected_field: u32      // index: 0
///
///     #[reflect(ignore)]
///     non_reflected_field: u32  // index: N/A (not 1!)
///
///     #[reflect(skip_serializing)]
///     non_serialized_field: u32 // index: 1
/// }
/// ```
/// Would convert to the `0b01` bitset (i.e second field is NOT serialized)
///
pub(crate) fn members_to_serialization_denylist<T>(member_iter: T) -> BitSet<u32>
where
    T: Iterator<Item = ReflectIgnoreBehavior>,
{
    let mut bitset = BitSet::default();

    member_iter.fold(0, |next_idx, member| match member {
        ReflectIgnoreBehavior::IgnoreAlways => next_idx,
        ReflectIgnoreBehavior::IgnoreSerialization => {
            bitset.insert(next_idx);
            next_idx + 1
        }
        ReflectIgnoreBehavior::None => next_idx + 1,
    });

    bitset
}
