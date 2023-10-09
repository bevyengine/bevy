//! General-purpose utility functions for internal usage within this crate.

use crate::derive_data::{ReflectMeta, StructField};
use crate::field_attributes::ReflectIgnoreBehavior;
use bevy_macro_utils::{
    fq_std::{FQAny, FQOption, FQSend, FQSync},
    BevyManifest,
};
use bit_set::BitSet;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, LitStr, Member, Path, Type, WhereClause};

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
    let reflected = format!("Reflect{name}");
    Ident::new(&reflected, Span::call_site())
}

/// Helper struct used to process an iterator of `Result<Vec<T>, syn::Error>`,
/// combining errors into one along the way.
pub(crate) struct ResultSifter<T> {
    items: Vec<T>,
    errors: Option<syn::Error>,
}

/// Returns a [`Member`] made of `ident` or `index` if `ident` is None.
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
    ident.map_or_else(
        || Member::Unnamed(index.into()),
        |ident| Member::Named(ident.clone()),
    )
}

/// Options defining how to extend the `where` clause in reflection with any additional bounds needed.
pub(crate) struct WhereClauseOptions {
    /// Type parameters that need extra trait bounds.
    parameter_types: Box<[Ident]>,
    /// Trait bounds to add to the type parameters.
    parameter_trait_bounds: Box<[proc_macro2::TokenStream]>,
    /// Any types that will be reflected and need an extra trait bound
    active_types: Box<[Type]>,
    /// Trait bounds to add to the active types
    active_trait_bounds: Box<[proc_macro2::TokenStream]>,
    /// Any types that won't be reflected and need an extra trait bound
    ignored_types: Box<[Type]>,
    /// Trait bounds to add to the ignored types
    ignored_trait_bounds: Box<[proc_macro2::TokenStream]>,
}

impl Default for WhereClauseOptions {
    /// By default, don't add any additional bounds to the `where` clause
    fn default() -> Self {
        Self {
            parameter_types: Box::new([]),
            active_types: Box::new([]),
            ignored_types: Box::new([]),
            active_trait_bounds: Box::new([]),
            ignored_trait_bounds: Box::new([]),
            parameter_trait_bounds: Box::new([]),
        }
    }
}

impl WhereClauseOptions {
    /// Create [`WhereClauseOptions`] for a struct or enum type.
    pub fn new<'a: 'b, 'b>(
        meta: &ReflectMeta,
        active_fields: impl Iterator<Item = &'b StructField<'a>>,
        ignored_fields: impl Iterator<Item = &'b StructField<'a>>,
    ) -> Self {
        Self::new_with_bounds(meta, active_fields, ignored_fields, |_| None, |_| None)
    }

    /// Create [`WhereClauseOptions`] for a simple value type.
    pub fn new_value(meta: &ReflectMeta) -> Self {
        Self::new_with_bounds(
            meta,
            std::iter::empty(),
            std::iter::empty(),
            |_| None,
            |_| None,
        )
    }

    /// Create [`WhereClauseOptions`] for a struct or enum type.
    ///
    /// Compared to [`WhereClauseOptions::new`], this version allows you to specify
    /// custom trait bounds for each field.
    pub fn new_with_bounds<'a: 'b, 'b>(
        meta: &ReflectMeta,
        active_fields: impl Iterator<Item = &'b StructField<'a>>,
        ignored_fields: impl Iterator<Item = &'b StructField<'a>>,
        active_bounds: impl Fn(&StructField<'a>) -> Option<proc_macro2::TokenStream>,
        ignored_bounds: impl Fn(&StructField<'a>) -> Option<proc_macro2::TokenStream>,
    ) -> Self {
        let bevy_reflect_path = meta.bevy_reflect_path();
        let is_from_reflect = meta.from_reflect().should_auto_derive();

        let (active_types, active_trait_bounds): (Vec<_>, Vec<_>) = active_fields
            .map(|field| {
                let ty = field.data.ty.clone();

                let custom_bounds = active_bounds(field).map(|bounds| quote!(+ #bounds));

                let bounds = if is_from_reflect {
                    quote!(#bevy_reflect_path::FromReflect #custom_bounds)
                } else {
                    quote!(#bevy_reflect_path::Reflect #custom_bounds)
                };

                (ty, bounds)
            })
            .unzip();

        let (ignored_types, ignored_trait_bounds): (Vec<_>, Vec<_>) = ignored_fields
            .map(|field| {
                let ty = field.data.ty.clone();

                let custom_bounds = ignored_bounds(field).map(|bounds| quote!(+ #bounds));
                let bounds = quote!(#FQAny + #FQSend + #FQSync #custom_bounds);

                (ty, bounds)
            })
            .unzip();

        let (parameter_types, parameter_trait_bounds): (Vec<_>, Vec<_>) =
            if meta.traits().type_path_attrs().should_auto_derive() {
                meta.type_path()
                    .generics()
                    .type_params()
                    .map(|param| {
                        let ident = param.ident.clone();
                        let bounds = quote!(#bevy_reflect_path::TypePath);
                        (ident, bounds)
                    })
                    .unzip()
            } else {
                // If we don't need to derive `TypePath` for the type parameters,
                // we can skip adding its bound to the `where` clause.
                (Vec::new(), Vec::new())
            };

        Self {
            active_types: active_types.into_boxed_slice(),
            active_trait_bounds: active_trait_bounds.into_boxed_slice(),
            ignored_types: ignored_types.into_boxed_slice(),
            ignored_trait_bounds: ignored_trait_bounds.into_boxed_slice(),
            parameter_types: parameter_types.into_boxed_slice(),
            parameter_trait_bounds: parameter_trait_bounds.into_boxed_slice(),
        }
    }
}

/// Extends the `where` clause in reflection with any additional bounds needed.
///
/// This is mostly used to add additional bounds to reflected objects with generic types.
/// For reflection purposes, we usually have:
/// * `active_trait_bounds: Reflect`
/// * `ignored_trait_bounds: Any + Send + Sync`
///
/// # Arguments
///
/// * `where_clause`: existing `where` clause present on the object to be derived
/// * `where_clause_options`: additional parameters defining which trait bounds to add to the `where` clause
///
/// # Example
///
/// The struct:
/// ```ignore
/// #[derive(Reflect)]
/// struct Foo<T, U> {
///     a: T,
///     #[reflect(ignore)]
///     b: U
/// }
/// ```
/// will have active types: `[T]` and ignored types: `[U]`
///
/// The `extend_where_clause` function will yield the following `where` clause:
/// ```ignore
/// where
///     T: Reflect,  // active_trait_bounds
///     U: Any + Send + Sync,  // ignored_trait_bounds
/// ```
pub(crate) fn extend_where_clause(
    where_clause: Option<&WhereClause>,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let parameter_types = &where_clause_options.parameter_types;
    let active_types = &where_clause_options.active_types;
    let ignored_types = &where_clause_options.ignored_types;
    let parameter_trait_bounds = &where_clause_options.parameter_trait_bounds;
    let active_trait_bounds = &where_clause_options.active_trait_bounds;
    let ignored_trait_bounds = &where_clause_options.ignored_trait_bounds;

    let mut generic_where_clause = if let Some(where_clause) = where_clause {
        let predicates = where_clause.predicates.iter();
        quote! {where #(#predicates,)*}
    } else if !(parameter_types.is_empty() && active_types.is_empty() && ignored_types.is_empty()) {
        quote! {where}
    } else {
        quote!()
    };

    // The nested parentheses here are required to properly scope HRTBs coming
    // from field types to the type itself, as the compiler will scope them to
    // the whole bound by default, resulting in a failure to prove trait
    // adherence.
    generic_where_clause.extend(quote! {
        #((#active_types): #active_trait_bounds,)*
        #((#ignored_types): #ignored_trait_bounds,)*
        // Leave parameter bounds to the end for more sane error messages.
        #((#parameter_types): #parameter_trait_bounds,)*
    });
    generic_where_clause
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

/// Converts an iterator over ignore behavior of members to a bitset of ignored members.
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

/// Turns an `Option<TokenStream>` into a `TokenStream` for an `Option`.
pub(crate) fn wrap_in_option(tokens: Option<proc_macro2::TokenStream>) -> proc_macro2::TokenStream {
    match tokens {
        Some(tokens) => quote! {
            #FQOption::Some(#tokens)
        },
        None => quote! {
            #FQOption::None
        },
    }
}

/// Contains tokens representing different kinds of string.
#[derive(Clone)]
pub(crate) enum StringExpr {
    /// A string that is valid at compile time.
    ///
    /// This is either a string literal like `"mystring"`,
    /// or a string created by a macro like [`module_path`]
    /// or [`concat`].
    Const(proc_macro2::TokenStream),
    /// A [string slice](str) that is borrowed for a `'static` lifetime.
    Borrowed(proc_macro2::TokenStream),
    /// An [owned string](String).
    Owned(proc_macro2::TokenStream),
}

impl<T: ToString + Spanned> From<T> for StringExpr {
    fn from(value: T) -> Self {
        Self::from_lit(&LitStr::new(&value.to_string(), value.span()))
    }
}

impl StringExpr {
    /// Creates a [constant] [`StringExpr`] from a [`struct@LitStr`].
    ///
    /// [constant]: StringExpr::Const
    pub fn from_lit(lit: &LitStr) -> Self {
        Self::Const(lit.to_token_stream())
    }

    /// Creates a [constant] [`StringExpr`] by interpreting a [string slice][str] as a [`struct@LitStr`].
    ///
    /// [constant]: StringExpr::Const
    pub fn from_str(string: &str) -> Self {
        Self::Const(string.into_token_stream())
    }

    /// Returns tokens for an [owned string](String).
    ///
    /// The returned expression will allocate unless the [`StringExpr`] is [already owned].
    ///
    /// [already owned]: StringExpr::Owned
    pub fn into_owned(self) -> proc_macro2::TokenStream {
        match self {
            Self::Const(tokens) | Self::Borrowed(tokens) => quote! {
                ::std::string::ToString::to_string(#tokens)
            },
            Self::Owned(owned) => owned,
        }
    }

    /// Returns tokens for a statically borrowed [string slice](str).
    pub fn into_borrowed(self) -> proc_macro2::TokenStream {
        match self {
            Self::Const(tokens) | Self::Borrowed(tokens) => tokens,
            Self::Owned(owned) => quote! {
                &#owned
            },
        }
    }

    /// Appends a [`StringExpr`] to another.
    ///
    /// If both expressions are [`StringExpr::Const`] this will use [`concat`] to merge them.
    pub fn appended_by(mut self, other: StringExpr) -> Self {
        if let Self::Const(tokens) = self {
            if let Self::Const(more) = other {
                return Self::Const(quote! {
                    ::core::concat!(#tokens, #more)
                });
            }
            self = Self::Const(tokens);
        }

        let owned = self.into_owned();
        let borrowed = other.into_borrowed();
        Self::Owned(quote! {
            #owned + #borrowed
        })
    }
}

impl Default for StringExpr {
    fn default() -> Self {
        StringExpr::from_str("")
    }
}

impl FromIterator<StringExpr> for StringExpr {
    fn from_iter<T: IntoIterator<Item = StringExpr>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        match iter.next() {
            Some(mut expr) => {
                for next in iter {
                    expr = expr.appended_by(next);
                }

                expr
            }
            None => Default::default(),
        }
    }
}
