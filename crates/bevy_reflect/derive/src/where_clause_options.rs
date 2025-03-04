use crate::derive_data::ReflectMeta;
use bevy_macro_utils::fq_std::{FQAny, FQSend, FQSync};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Token, Type, WhereClause};

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
    /// - Active fields have the bounds `TypePath` and either `PartialReflect` if `#[reflect(from_reflect = false)]` is present
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
    pub fn extend_where_clause(&self, where_clause: Option<&WhereClause>) -> TokenStream {
        // We would normally just use `Self`, but that won't work for generating things like assertion functions
        // and trait impls for a type's reference (e.g. `impl FromArg for &MyType`)
        let this = self.meta.type_path().true_type();

        let required_bounds = self.required_bounds();

        // Maintain existing where clause, if any.
        let mut generic_where_clause = if let Some(where_clause) = where_clause {
            let predicates = where_clause.predicates.iter();
            quote! {where #this: #required_bounds, #(#predicates,)*}
        } else {
            quote!(where #this: #required_bounds,)
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
                        // Needed for `Typed` impls
                        + #bevy_reflect_path::MaybeTyped
                        // Needed for `GetTypeRegistration` impls
                        + #bevy_reflect_path::__macro_exports::RegisterForReflection
                )
            }))
        }
    }

    /// The `PartialReflect` or `FromReflect` bound to use based on `#[reflect(from_reflect = false)]`.
    fn reflect_bound(&self) -> TokenStream {
        let bevy_reflect_path = self.meta.bevy_reflect_path();

        if self.meta.from_reflect().should_auto_derive() {
            quote!(#bevy_reflect_path::FromReflect)
        } else {
            quote!(#bevy_reflect_path::PartialReflect)
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
    fn required_bounds(&self) -> TokenStream {
        quote!(#FQAny + #FQSend + #FQSync)
    }
}
