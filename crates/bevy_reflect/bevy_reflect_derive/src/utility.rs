//! General-purpose utility functions for internal usage within this crate.

use crate::derive_data::ReflectMeta;
use bevy_macro_utils::{
    fq_std::{FQAny, FQOption, FQSend, FQSync},
    BevyManifest,
};
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, LitStr, Member, Path, TypeParam, WhereClause};

/// Returns the correct path for `bevy_reflect`.
pub(crate) fn get_bevy_reflect_path() -> Path {
    BevyManifest::get_path_direct("bevy_reflect")
}

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
    /// Any types that will be reflected and need an extra trait bound
    active_types: Vec<Ident>,
    /// Trait bounds to add to the active types
    active_trait_bounds: Vec<proc_macro2::TokenStream>,
    /// Any types that won't be reflected and need an extra trait bound
    ignored_types: Vec<Ident>,
    /// Trait bounds to add to the ignored types
    ignored_trait_bounds: Vec<proc_macro2::TokenStream>,
}

impl Default for WhereClauseOptions {
    /// By default, don't add any additional bounds to the `where` clause
    fn default() -> Self {
        Self {
            active_types: Vec::new(),
            ignored_types: Vec::new(),
            active_trait_bounds: Vec::new(),
            ignored_trait_bounds: Vec::new(),
        }
    }
}

impl WhereClauseOptions {
    /// Create [`WhereClauseOptions`] for a reflected struct or enum type.
    pub fn new(meta: &ReflectMeta) -> Result<Self, syn::Error> {
        let bevy_reflect_path = meta.bevy_reflect_path();

        let active_bound = if meta.from_reflect().should_auto_derive() {
            quote!(#bevy_reflect_path::FromReflect)
        } else {
            quote!(#bevy_reflect_path::Reflect)
        };

        let type_path_bound = if meta.traits().type_path_attrs().should_auto_derive() {
            Some(quote!(#bevy_reflect_path::TypePath +))
        } else {
            None
        };

        Self::new_with_bounds(
            meta,
            |_| Some(quote!(#type_path_bound #active_bound)),
            |_| Some(quote!(#type_path_bound #FQAny + #FQSend + #FQSync)),
        )
    }

    /// Create [`WhereClauseOptions`] with the minimum bounds needed to fulfill `TypePath`.
    pub fn new_type_path(meta: &ReflectMeta) -> Result<Self, syn::Error> {
        let bevy_reflect_path = meta.bevy_reflect_path();

        Self::new_with_bounds(
            meta,
            |_| Some(quote!(#bevy_reflect_path::TypePath)),
            |_| Some(quote!(#bevy_reflect_path::TypePath + #FQAny + #FQSend + #FQSync)),
        )
    }

    /// Create [`WhereClauseOptions`] for a struct or enum type.
    ///
    /// Compared to [`WhereClauseOptions::new`], this version allows you to specify
    /// custom trait bounds for each field.
    pub fn new_with_bounds(
        meta: &ReflectMeta,
        active_bounds: impl Fn(&TypeParam) -> Option<proc_macro2::TokenStream>,
        ignored_bounds: impl Fn(&TypeParam) -> Option<proc_macro2::TokenStream>,
    ) -> Result<Self, syn::Error> {
        let mut options = WhereClauseOptions::default();

        for param in meta.type_path().generics().type_params() {
            let ident = param.ident.clone();
            let ignored = meta.traits().ignore_param(&ident);

            if ignored {
                let bounds = ignored_bounds(param).unwrap_or_default();

                options.ignored_types.push(ident);
                options.ignored_trait_bounds.push(bounds);
            } else {
                let bounds = active_bounds(param).unwrap_or_default();

                options.active_types.push(ident);
                options.active_trait_bounds.push(bounds);
            }
        }

        Ok(options)
    }
}

/// Extends the `where` clause in reflection with any additional bounds needed.
///
/// This is mostly used to add additional bounds to reflected objects with generic types.
/// For reflection purposes, we usually have:
/// * `active_trait_bounds`: `Reflect + TypePath` or `FromReflect + TypePath`
/// * `ignored_trait_bounds`: `TypePath + Any + Send + Sync`
///
/// # Arguments
///
/// * `where_clause`: existing `where` clause present on the object to be derived
/// * `where_clause_options`: additional parameters defining which trait bounds to add to the `where` clause
///
/// # Example
///
/// The struct:
/// ```ignore (bevy_reflect is not accessible from this crate)
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
/// ```ignore (bevy_reflect is not accessible from this crate)
/// where
///     T: Reflect + TypePath,  // active_trait_bounds
///     U: TypePath + Any + Send + Sync,  // ignored_trait_bounds
/// ```
pub(crate) fn extend_where_clause(
    where_clause: Option<&WhereClause>,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let active_types = &where_clause_options.active_types;
    let ignored_types = &where_clause_options.ignored_types;
    let active_trait_bounds = &where_clause_options.active_trait_bounds;
    let ignored_trait_bounds = &where_clause_options.ignored_trait_bounds;

    let mut generic_where_clause = if let Some(where_clause) = where_clause {
        let predicates = where_clause.predicates.iter();
        quote! {where Self: 'static, #(#predicates,)*}
    } else {
        quote!(where Self: 'static,)
    };

    generic_where_clause.extend(quote! {
        #(#active_types: #active_trait_bounds,)*
        #(#ignored_types: #ignored_trait_bounds,)*
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
