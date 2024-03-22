//! General-purpose utility functions for internal usage within this crate.

use crate::derive_data::ReflectMeta;
use bevy_macro_utils::{
    fq_std::{FQAny, FQOption, FQSend, FQSync},
    BevyManifest,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Peek};
use syn::punctuated::Punctuated;
use syn::{spanned::Spanned, LitStr, Member, Path, Token, Type, WhereClause};

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
    active_fields: Box<[Type]>,
}

impl<'a, 'b> WhereClauseOptions<'a, 'b> {
    pub fn new(meta: &'a ReflectMeta<'b>) -> Self {
        Self {
            meta,
            active_fields: Box::new([]),
        }
    }

    pub fn new_with_fields(meta: &'a ReflectMeta<'b>, active_fields: Box<[Type]>) -> Self {
        Self {
            meta,
            active_fields,
        }
    }

    /// Extends the `where` clause for a type with additional bounds needed for the reflection impls.
    ///
    /// The default bounds added are as follows:
    /// - `Self` has the bounds `Any + Send + Sync`
    /// - Type parameters have the bound `TypePath` unless `#[reflect(type_path = false)]` is present
    /// - Active fields have the bounds `TypePath` and either `Reflect` if `#[reflect(from_reflect = false)]` is present
    ///   or `FromReflect` otherwise (or no bounds at all if `#[reflect(no_field_bounds)]` is present)
    ///
    /// When the derive is used with `#[reflect(where)]`, the bounds specified in the attribute are added as well.
    ///
    /// # Example
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
    /// Generates the following where clause:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// where
    ///   // `Self` bounds:
    ///   Self: Any + Send + Sync,
    ///   // Type parameter bounds:
    ///   T: TypePath,
    ///   U: TypePath,
    ///   // Field bounds
    ///   T: FromReflect + TypePath,
    /// ```
    ///
    /// If we had added `#[reflect(where T: MyTrait)]` to the type, it would instead generate:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// where
    ///   // `Self` bounds:
    ///   Self: Any + Send + Sync,
    ///   // Type parameter bounds:
    ///   T: TypePath,
    ///   U: TypePath,
    ///   // Field bounds
    ///   T: FromReflect + TypePath,
    ///   // Custom bounds
    ///   T: MyTrait,
    /// ```
    ///
    /// And if we also added `#[reflect(no_field_bounds)]` to the type, it would instead generate:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// where
    ///   // `Self` bounds:
    ///   Self: Any + Send + Sync,
    ///   // Type parameter bounds:
    ///   T: TypePath,
    ///   U: TypePath,
    ///   // Custom bounds
    ///   T: MyTrait,
    /// ```
    pub fn extend_where_clause(
        &self,
        where_clause: Option<&WhereClause>,
    ) -> proc_macro2::TokenStream {
        let required_bounds = self.required_bounds();
        // Maintain existing where clause, if any.
        let mut generic_where_clause = if let Some(where_clause) = where_clause {
            let predicates = where_clause.predicates.iter();
            quote! {where Self: #required_bounds, #(#predicates,)*}
        } else {
            quote!(where Self: #required_bounds,)
        };

        // Add additional reflection trait bounds
        let predicates = self.predicates();
        generic_where_clause.extend(quote! {
            #predicates
        });

        generic_where_clause
    }

    /// Returns an iterator the where clause predicates to extended the where clause with.
    fn predicates(&self) -> Punctuated<TokenStream, Token![,]> {
        let mut predicates = Punctuated::new();

        if let Some(type_param_predicates) = self.type_param_predicates() {
            predicates.extend(type_param_predicates);
        }

        if let Some(field_predicates) = self.active_field_predicates() {
            predicates.extend(field_predicates);
        }

        if let Some(custom_where) = self.meta.attrs().custom_where() {
            predicates.push(custom_where.predicates.to_token_stream());
        }

        predicates
    }

    /// Returns an iterator over the where clause predicates for the type parameters
    /// if they require one.
    fn type_param_predicates(&self) -> Option<impl Iterator<Item = TokenStream> + '_> {
        self.type_path_bound().map(|type_path_bound| {
            self.meta
                .type_path()
                .generics()
                .type_params()
                .map(move |param| {
                    let ident = &param.ident;

                    quote!(#ident : #type_path_bound)
                })
        })
    }

    /// Returns an iterator over the where clause predicates for the active fields.
    fn active_field_predicates(&self) -> Option<impl Iterator<Item = TokenStream> + '_> {
        if self.meta.attrs().no_field_bounds() {
            None
        } else {
            let bevy_reflect_path = self.meta.bevy_reflect_path();
            let reflect_bound = self.reflect_bound();

            // `TypePath` is always required for active fields since they are used to
            // construct `NamedField` and `UnnamedField` instances for the `Typed` impl.
            // Likewise, `GetTypeRegistration` is always required for active fields since
            // they are used to register the type's dependencies.
            Some(self.active_fields.iter().map(move |ty| {
                quote!(
                    #ty : #reflect_bound
                        + #bevy_reflect_path::TypePath
                        + #bevy_reflect_path::__macro_exports::RegisterForReflection
                )
            }))
        }
    }

    /// The `Reflect` or `FromReflect` bound to use based on `#[reflect(from_reflect = false)]`.
    fn reflect_bound(&self) -> TokenStream {
        let bevy_reflect_path = self.meta.bevy_reflect_path();

        if self.meta.from_reflect().should_auto_derive() {
            quote!(#bevy_reflect_path::FromReflect)
        } else {
            quote!(#bevy_reflect_path::Reflect)
        }
    }

    /// The `TypePath` bounds to use based on `#[reflect(type_path = false)]`.
    fn type_path_bound(&self) -> Option<TokenStream> {
        if self.meta.type_path_attrs().should_auto_derive() {
            let bevy_reflect_path = self.meta.bevy_reflect_path();
            Some(quote!(#bevy_reflect_path::TypePath))
        } else {
            None
        }
    }

    /// The minimum required bounds for a type to be reflected.
    fn required_bounds(&self) -> proc_macro2::TokenStream {
        quote!(#FQAny + #FQSend + #FQSync)
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

/// Returns a [`syn::parse::Parser`] which parses a stream of zero or more occurrences of `T`
/// separated by punctuation of type `P`, with optional trailing punctuation.
///
/// This is functionally the same as [`Punctuated::parse_terminated`],
/// but accepts a closure rather than a function pointer.
pub(crate) fn terminated_parser<T, P, F: FnMut(ParseStream) -> syn::Result<T>>(
    terminator: P,
    mut parser: F,
) -> impl FnOnce(ParseStream) -> syn::Result<Punctuated<T, P::Token>>
where
    P: Peek,
    P::Token: Parse,
{
    let _ = terminator;
    move |stream: ParseStream| {
        let mut punctuated = Punctuated::new();

        loop {
            if stream.is_empty() {
                break;
            }
            let value = parser(stream)?;
            punctuated.push_value(value);
            if stream.is_empty() {
                break;
            }
            let punct = stream.parse()?;
            punctuated.push_punct(punct);
        }

        Ok(punctuated)
    }
}
