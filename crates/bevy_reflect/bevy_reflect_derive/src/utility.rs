//! General-purpose utility functions for internal usage within this crate.

use crate::derive_data::ReflectMeta;
use bevy_macro_utils::{
    fq_std::{FQAny, FQOption, FQSend, FQSync},
    BevyManifest,
};
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, LitStr, Member, Path, WhereClause};

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

/// Options defining how to extend the `where` clause for reflection.
pub(crate) struct WhereClauseOptions<'a, 'b> {
    meta: &'a ReflectMeta<'b>,
    additional_bounds: proc_macro2::TokenStream,
    required_bounds: proc_macro2::TokenStream,
}

impl<'a, 'b> WhereClauseOptions<'a, 'b> {
    /// Create [`WhereClauseOptions`] for a reflected struct or enum type.
    pub fn new(meta: &'a ReflectMeta<'b>) -> Self {
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

        Self {
            meta,
            additional_bounds: quote!(#type_path_bound #active_bound),
            required_bounds: quote!(#type_path_bound #FQAny + #FQSend + #FQSync),
        }
    }

    /// Create [`WhereClauseOptions`] with the minimum bounds needed to fulfill `TypePath`.
    pub fn new_type_path(meta: &'a ReflectMeta<'b>) -> Self {
        let bevy_reflect_path = meta.bevy_reflect_path();

        Self {
            meta,
            additional_bounds: quote!(#bevy_reflect_path::TypePath),
            required_bounds: quote!(#bevy_reflect_path::TypePath + #FQAny + #FQSend + #FQSync),
        }
    }

    /// Extends the `where` clause in reflection with additional bounds needed for reflection.
    ///
    /// This will only add bounds for generic type parameters.
    ///
    /// If the container has a `#[reflect(custom_where(...))]` attribute,
    /// this method will extend the type parameters with the _required_ bounds.
    /// If the attribute is not present, it will extend the type parameters with the _additional_ bounds.
    ///
    /// The required bounds are the minimum bounds needed for a type to be reflected.
    /// These include `TypePath`, `Any`, `Send`, and `Sync`.
    ///
    /// The additional bounds are added bounds used to enforce that a generic type parameter
    /// is itself reflectable.
    /// These include `Reflect` and `FromReflect`, as well as `TypePath`.
    ///
    /// # Example
    ///
    /// Take the following struct:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// #[derive(Reflect)]
    /// struct Foo<T, U> {
    ///   a: T,
    ///   #[reflect(ignore)]
    ///   b: U
    /// }
    /// ```
    ///
    /// It has type parameters `T` and `U`.
    ///
    /// Since there is no `#[reflect(custom_where(...))]` attribute, this method will extend the type parameters
    /// with the additional bounds:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// where
    ///   T: FromReflect + TypePath, // additional bounds
    ///   U: FromReflect + TypePath, // additional bounds
    /// ```
    ///
    /// If we had this struct:
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// #[derive(Reflect)]
    /// #[reflect(custom_where(T: FromReflect + Default))]
    /// struct Foo<T, U> {
    ///   a: T,
    ///   #[reflect(ignore)]
    ///   b: U
    /// }
    /// ```
    ///
    /// Since there is a `#[reflect(custom_where(...))]` attribute, this method will extend the type parameters
    /// with _just_ the required bounds along with the predicates specified in the attribute:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// where
    ///   T: FromReflect + Default, // predicates from attribute
    ///   T: TypePath + Any + Send + Sync, // required bounds
    ///   U: TypePath + Any + Send + Sync, // required bounds
    /// ```
    pub fn extend_where_clause(
        &self,
        where_clause: Option<&WhereClause>,
    ) -> proc_macro2::TokenStream {
        // Maintain existing where clause, if any.
        let mut generic_where_clause = if let Some(where_clause) = where_clause {
            let predicates = where_clause.predicates.iter();
            quote! {where Self: 'static, #(#predicates,)*}
        } else {
            quote!(where Self: 'static,)
        };

        // Add additional reflection trait bounds
        let types = self.type_param_idents();
        let custom_where = self.meta.traits().custom_where();
        let trait_bounds = self.trait_bounds();

        generic_where_clause.extend(quote! {
            #(#types: #trait_bounds,)*
            #custom_where
        });

        generic_where_clause
    }

    /// Returns the trait bounds to use for all type parameters.
    fn trait_bounds(&self) -> &proc_macro2::TokenStream {
        if self.meta.traits().custom_where().is_some() {
            &self.required_bounds
        } else {
            &self.additional_bounds
        }
    }

    /// Returns an iterator of the type parameter idents for the reflected type.
    fn type_param_idents(&self) -> impl Iterator<Item = &Ident> {
        self.meta
            .type_path()
            .generics()
            .type_params()
            .map(|param| &param.ident)
    }
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
