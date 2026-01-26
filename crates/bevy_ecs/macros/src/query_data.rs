use bevy_macro_utils::{ensure_no_collision, get_struct_fields};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, DeriveInput, Meta,
};

use crate::{
    bevy_ecs_path,
    world_query::{item_struct, world_query_impl},
};

#[derive(Default)]
struct QueryDataAttributes {
    pub is_mutable: bool,

    pub derive_args: Punctuated<Meta, Comma>,
}

static MUTABLE_ATTRIBUTE_NAME: &str = "mutable";
static DERIVE_ATTRIBUTE_NAME: &str = "derive";

mod field_attr_keywords {
    syn::custom_keyword!(ignore);
}

pub static QUERY_DATA_ATTRIBUTE_NAME: &str = "query_data";

pub fn derive_query_data_impl(input: TokenStream) -> TokenStream {
    let tokens = input.clone();

    let ast = parse_macro_input!(input as DeriveInput);
    let visibility = ast.vis;

    let mut attributes = QueryDataAttributes::default();
    for attr in &ast.attrs {
        if !attr.path().is_ident(QUERY_DATA_ATTRIBUTE_NAME) {
            continue;
        }

        let result = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(MUTABLE_ATTRIBUTE_NAME) {
                attributes.is_mutable = true;
                Ok(())
            } else if meta.path.is_ident(DERIVE_ATTRIBUTE_NAME) {
                meta.parse_nested_meta(|meta| {
                    attributes.derive_args.push(Meta::Path(meta.path));
                    Ok(())
                })
            } else {
                Err(meta.error(format_args!("invalid attribute, expected `{MUTABLE_ATTRIBUTE_NAME}` or `{DERIVE_ATTRIBUTE_NAME}`")))
            }
        });

        if let Err(err) = result {
            return err.to_compile_error().into();
        }
    }

    let path = bevy_ecs_path();

    let user_generics = ast.generics.clone();
    let (user_impl_generics, user_ty_generics, user_where_clauses) = user_generics.split_for_impl();
    let user_generics_with_world = {
        let mut generics = ast.generics.clone();
        generics.params.insert(0, parse_quote!('__w));
        generics
    };
    let (user_impl_generics_with_world, user_ty_generics_with_world, user_where_clauses_with_world) =
        user_generics_with_world.split_for_impl();
    let user_generics_with_world_and_state = {
        let mut generics = ast.generics;
        generics.params.insert(0, parse_quote!('__w));
        generics.params.insert(1, parse_quote!('__s));
        generics
    };
    let (
        user_impl_generics_with_world_and_state,
        user_ty_generics_with_world_and_state,
        user_where_clauses_with_world_and_state,
    ) = user_generics_with_world_and_state.split_for_impl();

    let struct_name = ast.ident;
    let read_only_struct_name = if attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnly"), Span::call_site())
    } else {
        struct_name.clone()
    };

    let item_struct_name = Ident::new(&format!("{struct_name}Item"), Span::call_site());
    let read_only_item_struct_name = if attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnlyItem"), Span::call_site())
    } else {
        item_struct_name.clone()
    };

    let fetch_struct_name = Ident::new(&format!("{struct_name}Fetch"), Span::call_site());
    let fetch_struct_name = ensure_no_collision(fetch_struct_name, tokens.clone());
    let read_only_fetch_struct_name = if attributes.is_mutable {
        let new_ident = Ident::new(&format!("{struct_name}ReadOnlyFetch"), Span::call_site());
        ensure_no_collision(new_ident, tokens.clone())
    } else {
        fetch_struct_name.clone()
    };

    let marker_name =
        ensure_no_collision(format_ident!("_world_query_derive_marker"), tokens.clone());

    // Generate a name for the state struct that doesn't conflict
    // with the struct definition.
    let state_struct_name = Ident::new(&format!("{struct_name}State"), Span::call_site());
    let state_struct_name = ensure_no_collision(state_struct_name, tokens);

    let fields = match get_struct_fields(&ast.data, "derive(QueryData)") {
        Ok(fields) => fields,
        Err(e) => return e.into_compile_error().into(),
    };

    let field_attrs = fields.iter().map(|f| f.attrs.clone()).collect();
    let field_visibilities = fields.iter().map(|f| f.vis.clone()).collect();
    let field_members = fields.members().collect();
    let field_aliases = fields
        .members()
        .map(|m| format_ident!("field{}", m))
        .collect();
    let field_types: Vec<syn::Type> = fields.iter().map(|f| f.ty.clone()).collect();
    let read_only_field_types = field_types
        .iter()
        .map(|ty| parse_quote!(<#ty as #path::query::QueryData>::ReadOnly))
        .collect();

    let derive_args = &attributes.derive_args;
    // `#[derive()]` is valid syntax
    let derive_macro_call = quote! { #[derive(#derive_args)] };

    let mutable_item_struct = item_struct(
        &path,
        fields,
        &derive_macro_call,
        &struct_name,
        &visibility,
        &item_struct_name,
        &field_types,
        &user_impl_generics_with_world_and_state,
        &field_attrs,
        &field_visibilities,
        &field_members,
        &user_ty_generics,
        &user_ty_generics_with_world_and_state,
        user_where_clauses_with_world_and_state,
    );
    let mutable_world_query_impl = world_query_impl(
        &path,
        &struct_name,
        &visibility,
        &fetch_struct_name,
        &field_types,
        &user_impl_generics,
        &user_impl_generics_with_world,
        &user_ty_generics,
        &user_ty_generics_with_world,
        &field_aliases,
        &marker_name,
        &state_struct_name,
        user_where_clauses,
        user_where_clauses_with_world,
    );

    let (read_only_struct, read_only_impl) = if attributes.is_mutable {
        // If the query is mutable, we need to generate a separate readonly version of some things
        let readonly_item_struct = item_struct(
            &path,
            fields,
            &derive_macro_call,
            &read_only_struct_name,
            &visibility,
            &read_only_item_struct_name,
            &read_only_field_types,
            &user_impl_generics_with_world_and_state,
            &field_attrs,
            &field_visibilities,
            &field_members,
            &user_ty_generics,
            &user_ty_generics_with_world_and_state,
            user_where_clauses_with_world_and_state,
        );
        let readonly_world_query_impl = world_query_impl(
            &path,
            &read_only_struct_name,
            &visibility,
            &read_only_fetch_struct_name,
            &read_only_field_types,
            &user_impl_generics,
            &user_impl_generics_with_world,
            &user_ty_generics,
            &user_ty_generics_with_world,
            &field_aliases,
            &marker_name,
            &state_struct_name,
            user_where_clauses,
            user_where_clauses_with_world,
        );
        let read_only_structs = quote! {
            #[doc = concat!(
                "Automatically generated [`WorldQuery`](",
                stringify!(#path),
                "::query::WorldQuery) type for a read-only variant of [`",
                stringify!(#struct_name),
                "`]."
            )]
            #[automatically_derived]
            #visibility struct #read_only_struct_name #user_impl_generics #user_where_clauses {
                #(
                    #[doc = "Automatically generated read-only field for accessing `"]
                    #[doc = stringify!(#field_types)]
                    #[doc = "`."]
                    #field_visibilities #field_members: #read_only_field_types,
                )*
            }

            #readonly_item_struct
        };
        (read_only_structs, readonly_world_query_impl)
    } else {
        (quote! {}, quote! {})
    };

    let data_impl = {
        let read_only_data_impl = if attributes.is_mutable {
            quote! {
                /// SAFETY: we assert fields are readonly below
                unsafe impl #user_impl_generics #path::query::QueryData
                for #read_only_struct_name #user_ty_generics #user_where_clauses {
                    const IS_READ_ONLY: bool = true;
                    type ReadOnly = #read_only_struct_name #user_ty_generics;
                    type Item<'__w, '__s> = #read_only_item_struct_name #user_ty_generics_with_world_and_state;

                    fn shrink<'__wlong: '__wshort, '__wshort, '__s>(
                        item: Self::Item<'__wlong, '__s>
                    ) -> Self::Item<'__wshort, '__s> {
                        #read_only_item_struct_name {
                            #(
                                #field_members: <#read_only_field_types>::shrink(item.#field_members),
                            )*
                        }
                    }

                    fn provide_extra_access(
                        state: &mut Self::State,
                        access: &mut #path::query::Access,
                        available_access: &#path::query::Access,
                    ) {
                        #(<#field_types>::provide_extra_access(&mut state.#field_aliases, access, available_access);)*
                    }

                    /// SAFETY: we call `fetch` for each member that implements `Fetch`.
                    #[inline(always)]
                    unsafe fn fetch<'__w, '__s>(
                        _state: &'__s Self::State,
                        _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                        _entity: #path::entity::Entity,
                        _table_row: #path::storage::TableRow,
                    ) -> Self::Item<'__w, '__s> {
                        Self::Item {
                            #(#field_members: <#read_only_field_types>::fetch(&_state.#field_aliases, &mut _fetch.#field_aliases, _entity, _table_row),)*
                        }
                    }

                    fn iter_access(
                        _state: &Self::State,
                    ) -> impl core::iter::Iterator<Item = #path::query::EcsAccessType<'_>> {
                        core::iter::empty() #(.chain(<#field_types>::iter_access(&_state.#field_aliases)))*
                    }
                }

                impl #user_impl_generics #path::query::ReleaseStateQueryData
                for #read_only_struct_name #user_ty_generics #user_where_clauses
                // Make these HRTBs with an unused lifetime parameter to allow trivial constraints
                // See https://github.com/rust-lang/rust/issues/48214
                where #(for<'__a> #field_types: #path::query::QueryData<ReadOnly: #path::query::ReleaseStateQueryData>,)* {
                    fn release_state<'__w>(_item: Self::Item<'__w, '_>) -> Self::Item<'__w, 'static> {
                        Self::Item {
                            #(#field_members: <#read_only_field_types>::release_state(_item.#field_members),)*
                        }
                    }
                }

                impl #user_impl_generics #path::query::ArchetypeQueryData
                for #read_only_struct_name #user_ty_generics #user_where_clauses
                // Make these HRTBs with an unused lifetime parameter to allow trivial constraints
                // See https://github.com/rust-lang/rust/issues/48214
                where #(for<'__a> #field_types: #path::query::ArchetypeQueryData,)* {}
            }
        } else {
            quote! {}
        };

        let is_read_only = !attributes.is_mutable;

        quote! {
            /// SAFETY: we assert fields are readonly below
            unsafe impl #user_impl_generics #path::query::QueryData
            for #struct_name #user_ty_generics #user_where_clauses {
                const IS_READ_ONLY: bool = #is_read_only;
                type ReadOnly = #read_only_struct_name #user_ty_generics;
                type Item<'__w, '__s> = #item_struct_name #user_ty_generics_with_world_and_state;

                fn shrink<'__wlong: '__wshort, '__wshort, '__s>(
                    item: Self::Item<'__wlong, '__s>
                ) -> Self::Item<'__wshort, '__s> {
                    #item_struct_name {
                        #(
                            #field_members: <#field_types>::shrink(item.#field_members),
                        )*
                    }
                }

                fn provide_extra_access(
                    state: &mut Self::State,
                    access: &mut #path::query::Access,
                    available_access: &#path::query::Access,
                ) {
                    #(<#field_types>::provide_extra_access(&mut state.#field_aliases, access, available_access);)*
                }

                /// SAFETY: we call `fetch` for each member that implements `Fetch`.
                #[inline(always)]
                unsafe fn fetch<'__w, '__s>(
                    _state: &'__s Self::State,
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _entity: #path::entity::Entity,
                    _table_row: #path::storage::TableRow,
                ) -> Self::Item<'__w, '__s> {
                    Self::Item {
                        #(#field_members: <#field_types>::fetch(&_state.#field_aliases, &mut _fetch.#field_aliases, _entity, _table_row),)*
                    }
                }

                fn iter_access(
                    _state: &Self::State,
                ) -> impl core::iter::Iterator<Item = #path::query::EcsAccessType<'_>> {
                    core::iter::empty() #(.chain(<#field_types>::iter_access(&_state.#field_aliases)))*
                }
            }

            impl #user_impl_generics #path::query::ReleaseStateQueryData
            for #struct_name #user_ty_generics #user_where_clauses
            // Make these HRTBs with an unused lifetime parameter to allow trivial constraints
            // See https://github.com/rust-lang/rust/issues/48214
            where #(for<'__a> #field_types: #path::query::ReleaseStateQueryData,)* {
                fn release_state<'__w>(_item: Self::Item<'__w, '_>) -> Self::Item<'__w, 'static> {
                    Self::Item {
                        #(#field_members: <#field_types>::release_state(_item.#field_members),)*
                    }
                }
            }

            impl #user_impl_generics #path::query::ArchetypeQueryData
            for #struct_name #user_ty_generics #user_where_clauses
            // Make these HRTBs with an unused lifetime parameter to allow trivial constraints
            // See https://github.com/rust-lang/rust/issues/48214
            where #(for<'__a> #field_types: #path::query::ArchetypeQueryData,)* {}

            #read_only_data_impl
        }
    };

    let read_only_data_impl = quote! {
        /// SAFETY: we assert fields are readonly below
        unsafe impl #user_impl_generics #path::query::ReadOnlyQueryData
        for #read_only_struct_name #user_ty_generics #user_where_clauses {}
    };

    let read_only_asserts = if attributes.is_mutable {
        quote! {
            // Double-check that the data fetched by `<_ as WorldQuery>::ReadOnly` is read-only.
            // This is technically unnecessary as `<_ as WorldQuery>::ReadOnly: ReadOnlyQueryData`
            // but to protect against future mistakes we assert the assoc type implements `ReadOnlyQueryData` anyway
            #( assert_readonly::<#read_only_field_types>(); )*
        }
    } else {
        quote! {
            // Statically checks that the safety guarantee of `ReadOnlyQueryData` for `$fetch_struct_name` actually holds true.
            // We need this to make sure that we don't compile `ReadOnlyQueryData` if our struct contains nested `QueryData`
            // members that don't implement it. I.e.:
            // ```
            // #[derive(QueryData)]
            // pub struct Foo { a: &'static mut MyComponent }
            // ```
            #( assert_readonly::<#field_types>(); )*
        }
    };

    let data_asserts = quote! {
        #( assert_data::<#field_types>(); )*
    };

    TokenStream::from(quote! {
        #mutable_item_struct

        #read_only_struct

        const _: () = {
            #[doc(hidden)]
            #[doc = concat!(
                "Automatically generated internal [`WorldQuery`](",
                stringify!(#path),
                "::query::WorldQuery) state type for [`",
                stringify!(#struct_name),
                "`], used for caching."
            )]
            #[automatically_derived]
            #visibility struct #state_struct_name #user_impl_generics #user_where_clauses {
                #(#field_aliases: <#field_types as #path::query::WorldQuery>::State,)*
            }

            #mutable_world_query_impl

            #read_only_impl

            #data_impl

            #read_only_data_impl
        };

        #[allow(dead_code)]
        const _: () = {
            fn assert_readonly<T>()
            where
                T: #path::query::ReadOnlyQueryData,
            {
            }

            fn assert_data<T>()
            where
                T: #path::query::QueryData,
            {
            }

            // We generate a readonly assertion for every struct member.
            fn assert_all #user_impl_generics_with_world () #user_where_clauses_with_world {
                #read_only_asserts
                #data_asserts
            }
        };

        // The original struct will most likely be left unused. As we don't want our users having
        // to specify `#[allow(dead_code)]` for their custom queries, we are using this cursed
        // workaround.
        #[allow(dead_code)]
        const _: () = {
            fn dead_code_workaround #user_impl_generics (
                q: #struct_name #user_ty_generics,
                q2: #read_only_struct_name #user_ty_generics
            ) #user_where_clauses {
                #(q.#field_members;)*
                #(q2.#field_members;)*
            }
        };
    })
}
