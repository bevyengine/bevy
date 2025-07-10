use crate::derive_data::ReflectMeta;
use bevy_macro_utils::fq_std::{FQAny, FQSend, FQSync};
use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Ident, Token, Type, WhereClause};

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

    pub fn meta(&self) -> &'a ReflectMeta<'b> {
        self.meta
    }

    /// Extends the `where` clause for a type with additional bounds needed for the reflection
    /// impls.
    ///
    /// The default bounds added are as follows:
    /// - `Self` has:
    ///   - `Any + Send + Sync` bounds, if generic over types
    ///   - An `Any` bound, if generic over lifetimes but not types
    ///   - No bounds, if generic over neither types nor lifetimes
    /// - Any given bounds in a `where` clause on the type
    /// - Type parameters have the bound `TypePath` unless `#[reflect(type_path = false)]` is
    ///   present
    /// - Active fields with non-generic types have the bounds `TypePath`, either `PartialReflect`
    ///   if `#[reflect(from_reflect = false)]` is present or `FromReflect` otherwise,
    ///   `MaybeTyped`, and `RegisterForReflection` (or no bounds at all if
    ///   `#[reflect(no_field_bounds)]` is present)
    ///
    /// When the derive is used with `#[reflect(where)]`, the bounds specified in the attribute are
    /// added as well.
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
    ///   Foo<T, U>: Any + Send + Sync,
    ///   // Type parameter bounds:
    ///   T: TypePath,
    ///   U: TypePath,
    ///   // Active non-generic field bounds
    ///   T: FromReflect + TypePath + MaybeTyped + RegisterForReflection,
    ///
    /// ```
    ///
    /// If we add various things to the type:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// #[derive(Reflect)]
    /// #[reflect(where T: MyTrait)]
    /// #[reflect(no_field_bounds)]
    /// struct Foo<T, U>
    ///     where T: Clone
    /// {
    ///   a: T,
    ///   #[reflect(ignore)]
    ///   b: U
    /// }
    /// ```
    ///
    /// It will instead generate the following where clause:
    ///
    /// ```ignore (bevy_reflect is not accessible from this crate)
    /// where
    ///   // `Self` bounds:
    ///   Foo<T, U>: Any + Send + Sync,
    ///   // Given bounds:
    ///   T: Clone,
    ///   // Type parameter bounds:
    ///   T: TypePath,
    ///   U: TypePath,
    ///   // No active non-generic field bounds
    ///   // Custom bounds
    ///   T: MyTrait,
    /// ```
    pub fn extend_where_clause(&self, where_clause: Option<&WhereClause>) -> TokenStream {
        let mut generic_where_clause = quote! { where };

        // Bounds on `Self`. We would normally just use `Self`, but that won't work for generating
        // things like assertion functions and trait impls for a type's reference (e.g. `impl
        // FromArg for &MyType`).
        let generics = self.meta.type_path().generics();
        if generics.type_params().next().is_some() {
            // Generic over types? We need `Any + Send + Sync`.
            let this = self.meta.type_path().true_type();
            generic_where_clause.extend(quote! { #this: #FQAny + #FQSend + #FQSync, });
        } else if generics.lifetimes().next().is_some() {
            // Generic only over lifetimes? We need `'static`.
            let this = self.meta.type_path().true_type();
            generic_where_clause.extend(quote! { #this: 'static, });
        }

        // Maintain existing where clause bounds, if any.
        if let Some(where_clause) = where_clause {
            let predicates = where_clause.predicates.iter();
            generic_where_clause.extend(quote! { #(#predicates,)* });
        }

        // Add additional reflection trait bounds.
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

            // Get the identifiers of all type parameters.
            let type_param_idents = self
                .meta
                .type_path()
                .generics()
                .type_params()
                .map(|type_param| type_param.ident.clone())
                .collect::<Vec<Ident>>();

            // Do any of the identifiers in `idents` appear in `token_stream`?
            fn is_any_ident_in_token_stream(idents: &[Ident], token_stream: TokenStream) -> bool {
                for token_tree in token_stream {
                    match token_tree {
                        TokenTree::Ident(ident) => {
                            if idents.contains(&ident) {
                                return true;
                            }
                        }
                        TokenTree::Group(group) => {
                            if is_any_ident_in_token_stream(idents, group.stream()) {
                                return true;
                            }
                        }
                        TokenTree::Punct(_) | TokenTree::Literal(_) => {}
                    }
                }
                false
            }

            Some(self.active_fields.iter().filter_map(move |ty| {
                // Field type bounds are only required if `ty` is generic. How to determine that?
                // Search `ty`s token stream for identifiers that match the identifiers from the
                // function's type params. E.g. if `T` and `U` are the type param identifiers and
                // `ty` is `Vec<[T; 4]>` then the `T` identifiers match. This is a bit hacky, but
                // it works.
                let is_generic =
                    is_any_ident_in_token_stream(&type_param_idents, ty.to_token_stream());

                is_generic.then(|| {
                    quote!(
                        #ty: #reflect_bound
                            // Needed to construct `NamedField` and `UnnamedField` instances for
                            // the `Typed` impl.
                            + #bevy_reflect_path::TypePath
                            // Needed for `Typed` impls
                            + #bevy_reflect_path::MaybeTyped
                            // Needed for registering type dependencies in the
                            // `GetTypeRegistration` impl.
                            + #bevy_reflect_path::__macro_exports::RegisterForReflection
                    )
                })
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
}
