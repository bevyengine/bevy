use bevy_macro_utils::ensure_no_collision;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::Comma,
    Attribute, Data, DataStruct, DeriveInput, Field, Index, Meta,
};

use crate::bevy_ecs_path;

#[derive(Default)]
struct FetchStructAttributes {
    pub is_mutable: bool,
    pub derive_args: Punctuated<syn::Meta, syn::token::Comma>,
}

static MUTABLE_ATTRIBUTE_NAME: &str = "mutable";
static DERIVE_ATTRIBUTE_NAME: &str = "derive";

mod field_attr_keywords {
    syn::custom_keyword!(ignore);
}

pub static WORLD_QUERY_ATTRIBUTE_NAME: &str = "world_query";

pub fn derive_world_query_impl(input: TokenStream) -> TokenStream {
    let tokens = input.clone();

    let ast = parse_macro_input!(input as DeriveInput);
    let visibility = ast.vis;

    let mut fetch_struct_attributes = FetchStructAttributes::default();
    for attr in &ast.attrs {
        if !attr
            .path()
            .get_ident()
            .map_or(false, |ident| ident == WORLD_QUERY_ATTRIBUTE_NAME)
        {
            continue;
        }

        attr.parse_args_with(|input: ParseStream| {
            let meta = input.parse_terminated(syn::Meta::parse, Comma)?;
            for meta in meta {
                let ident = meta.path().get_ident().unwrap_or_else(|| {
                    panic!(
                        "Unrecognized attribute: `{}`",
                        meta.path().to_token_stream()
                    )
                });
                if ident == MUTABLE_ATTRIBUTE_NAME {
                    if let syn::Meta::Path(_) = meta {
                        fetch_struct_attributes.is_mutable = true;
                    } else {
                        panic!(
                            "The `{MUTABLE_ATTRIBUTE_NAME}` attribute is expected to have no value or arguments",
                        );
                    }
                } else if ident == DERIVE_ATTRIBUTE_NAME {
                    if let syn::Meta::List(meta_list) = meta {
                        meta_list.parse_nested_meta(|meta| {
                            fetch_struct_attributes.derive_args.push(Meta::Path(meta.path));
                            Ok(())
                        })?;
                    } else {
                        panic!(
                            "Expected a structured list within the `{DERIVE_ATTRIBUTE_NAME}` attribute",
                        );
                    }
                } else {
                    panic!(
                        "Unrecognized attribute: `{}`",
                        meta.path().to_token_stream()
                    );
                }
            }
            Ok(())
        })
        .unwrap_or_else(|_| panic!("Invalid `{WORLD_QUERY_ATTRIBUTE_NAME}` attribute format"));
    }

    let path = bevy_ecs_path();

    let user_generics = ast.generics.clone();
    let (user_impl_generics, user_ty_generics, user_where_clauses) = user_generics.split_for_impl();
    let user_generics_with_world = {
        let mut generics = ast.generics;
        generics.params.insert(0, parse_quote!('__w));
        generics
    };
    let (user_impl_generics_with_world, user_ty_generics_with_world, user_where_clauses_with_world) =
        user_generics_with_world.split_for_impl();

    let struct_name = ast.ident;
    let read_only_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnly"), Span::call_site())
    } else {
        #[allow(clippy::redundant_clone)]
        struct_name.clone()
    };

    let item_struct_name = Ident::new(&format!("{struct_name}Item"), Span::call_site());
    let read_only_item_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnlyItem"), Span::call_site())
    } else {
        #[allow(clippy::redundant_clone)]
        item_struct_name.clone()
    };

    let fetch_struct_name = Ident::new(&format!("{struct_name}Fetch"), Span::call_site());
    let fetch_struct_name = ensure_no_collision(fetch_struct_name, tokens.clone());
    let read_only_fetch_struct_name = if fetch_struct_attributes.is_mutable {
        let new_ident = Ident::new(&format!("{struct_name}ReadOnlyFetch"), Span::call_site());
        ensure_no_collision(new_ident, tokens.clone())
    } else {
        #[allow(clippy::redundant_clone)]
        fetch_struct_name.clone()
    };

    let marker_name =
        ensure_no_collision(format_ident!("_world_query_derive_marker"), tokens.clone());

    // Generate a name for the state struct that doesn't conflict
    // with the struct definition.
    let state_struct_name = Ident::new(&format!("{struct_name}State"), Span::call_site());
    let state_struct_name = ensure_no_collision(state_struct_name, tokens);

    let Data::Struct(DataStruct { fields, .. }) = &ast.data else {
        return syn::Error::new(
            Span::call_site(),
            "#[derive(WorldQuery)]` only supports structs",
        )
        .into_compile_error()
        .into();
    };

    let mut field_attrs = Vec::new();
    let mut field_visibilities = Vec::new();
    let mut field_idents = Vec::new();
    let mut named_field_idents = Vec::new();
    let mut field_types = Vec::new();
    let mut read_only_field_types = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let attrs = match read_world_query_field_info(field) {
            Ok(WorldQueryFieldInfo { attrs }) => attrs,
            Err(e) => return e.into_compile_error().into(),
        };

        let named_field_ident = field
            .ident
            .as_ref()
            .cloned()
            .unwrap_or_else(|| format_ident!("f{i}"));
        let i = Index::from(i);
        let field_ident = field
            .ident
            .as_ref()
            .map_or(quote! { #i }, |i| quote! { #i });
        field_idents.push(field_ident);
        named_field_idents.push(named_field_ident);
        field_attrs.push(attrs);
        field_visibilities.push(field.vis.clone());
        let field_ty = field.ty.clone();
        field_types.push(quote!(#field_ty));
        read_only_field_types.push(quote!(<#field_ty as #path::query::WorldQuery>::ReadOnly));
    }

    let derive_args = &fetch_struct_attributes.derive_args;
    // `#[derive()]` is valid syntax
    let derive_macro_call = quote! { #[derive(#derive_args)] };

    let impl_fetch = |is_readonly: bool| {
        let struct_name = if is_readonly {
            &read_only_struct_name
        } else {
            &struct_name
        };
        let item_struct_name = if is_readonly {
            &read_only_item_struct_name
        } else {
            &item_struct_name
        };
        let fetch_struct_name = if is_readonly {
            &read_only_fetch_struct_name
        } else {
            &fetch_struct_name
        };

        let field_types = if is_readonly {
            &read_only_field_types
        } else {
            &field_types
        };

        let item_struct = match fields {
            syn::Fields::Named(_) => quote! {
                #derive_macro_call
                #[doc = "Automatically generated [`WorldQuery`] item type for [`"]
                #[doc = stringify!(#struct_name)]
                #[doc = "`], returned when iterating over query results."]
                #[automatically_derived]
                #visibility struct #item_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                    #(#(#field_attrs)* #field_visibilities #field_idents: <#field_types as #path::query::WorldQuery>::Item<'__w>,)*
                }
            },
            syn::Fields::Unnamed(_) => quote! {
                #derive_macro_call
                #[doc = "Automatically generated [`WorldQuery`] item type for [`"]
                #[doc = stringify!(#struct_name)]
                #[doc = "`], returned when iterating over query results."]
                #[automatically_derived]
                #visibility struct #item_struct_name #user_impl_generics_with_world #user_where_clauses_with_world(
                    #( #field_visibilities <#field_types as #path::query::WorldQuery>::Item<'__w>, )*
                );
            },
            syn::Fields::Unit => quote! {
                #[doc = "Automatically generated [`WorldQuery`] item type for [`"]
                #[doc = stringify!(#struct_name)]
                #[doc = "`], returned when iterating over query results."]
                #[automatically_derived]
                #visibility type #item_struct_name #user_ty_generics_with_world = #struct_name #user_ty_generics;
            },
        };

        let query_impl = quote! {
            #[doc(hidden)]
            #[doc = "Automatically generated internal [`WorldQuery`] fetch type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], used to define the world data accessed by this query."]
            #[automatically_derived]
            #visibility struct #fetch_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                #(#named_field_idents: <#field_types as #path::query::WorldQuery>::Fetch<'__w>,)*
                #marker_name: &'__w (),
            }

            impl #user_impl_generics_with_world Clone for #fetch_struct_name #user_ty_generics_with_world
            #user_where_clauses_with_world {
                fn clone(&self) -> Self {
                    Self {
                        #(#named_field_idents: self.#named_field_idents.clone(),)*
                        #marker_name: &(),
                    }
                }
            }

            // SAFETY: `update_component_access` and `update_archetype_component_access` are called on every field
            unsafe impl #user_impl_generics #path::query::WorldQuery
                for #struct_name #user_ty_generics #user_where_clauses {

                type Item<'__w> = #item_struct_name #user_ty_generics_with_world;
                type Fetch<'__w> = #fetch_struct_name #user_ty_generics_with_world;
                type ReadOnly = #read_only_struct_name #user_ty_generics;
                type State = #state_struct_name #user_ty_generics;

                fn shrink<'__wlong: '__wshort, '__wshort>(
                    item: <#struct_name #user_ty_generics as #path::query::WorldQuery>::Item<'__wlong>
                ) -> <#struct_name #user_ty_generics as #path::query::WorldQuery>::Item<'__wshort> {
                    #item_struct_name {
                        #(
                            #field_idents: <#field_types>::shrink(item.#field_idents),
                        )*
                    }
                }

                unsafe fn init_fetch<'__w>(
                    _world: #path::world::unsafe_world_cell::UnsafeWorldCell<'__w>,
                    state: &Self::State,
                    _last_run: #path::component::Tick,
                    _this_run: #path::component::Tick,
                ) -> <Self as #path::query::WorldQuery>::Fetch<'__w> {
                    #fetch_struct_name {
                        #(#named_field_idents:
                            <#field_types>::init_fetch(
                                _world,
                                &state.#named_field_idents,
                                _last_run,
                                _this_run,
                            ),
                        )*
                        #marker_name: &(),
                    }
                }

                const IS_DENSE: bool = true #(&& <#field_types>::IS_DENSE)*;

                const IS_ARCHETYPAL: bool = true #(&& <#field_types>::IS_ARCHETYPAL)*;

                /// SAFETY: we call `set_archetype` for each member that implements `Fetch`
                #[inline]
                unsafe fn set_archetype<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _state: &Self::State,
                    _archetype: &'__w #path::archetype::Archetype,
                    _table: &'__w #path::storage::Table
                ) {
                    #(<#field_types>::set_archetype(&mut _fetch.#named_field_idents, &_state.#named_field_idents, _archetype, _table);)*
                }

                /// SAFETY: we call `set_table` for each member that implements `Fetch`
                #[inline]
                unsafe fn set_table<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _state: &Self::State,
                    _table: &'__w #path::storage::Table
                ) {
                    #(<#field_types>::set_table(&mut _fetch.#named_field_idents, &_state.#named_field_idents, _table);)*
                }

                /// SAFETY: we call `fetch` for each member that implements `Fetch`.
                #[inline(always)]
                unsafe fn fetch<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _entity: #path::entity::Entity,
                    _table_row: #path::storage::TableRow,
                ) -> <Self as #path::query::WorldQuery>::Item<'__w> {
                    Self::Item {
                        #(#field_idents: <#field_types>::fetch(&mut _fetch.#named_field_idents, _entity, _table_row),)*
                    }
                }

                #[allow(unused_variables)]
                #[inline(always)]
                unsafe fn filter_fetch<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _entity: #path::entity::Entity,
                    _table_row: #path::storage::TableRow,
                ) -> bool {
                    true #(&& <#field_types>::filter_fetch(&mut _fetch.#named_field_idents, _entity, _table_row))*
                }

                fn update_component_access(state: &Self::State, _access: &mut #path::query::FilteredAccess<#path::component::ComponentId>) {
                    #( <#field_types>::update_component_access(&state.#named_field_idents, _access); )*
                }

                fn update_archetype_component_access(
                    state: &Self::State,
                    _archetype: &#path::archetype::Archetype,
                    _access: &mut #path::query::Access<#path::archetype::ArchetypeComponentId>
                ) {
                    #(
                        <#field_types>::update_archetype_component_access(&state.#named_field_idents, _archetype, _access);
                    )*
                }

                fn init_state(world: &mut #path::world::World) -> #state_struct_name #user_ty_generics {
                    #state_struct_name {
                        #(#named_field_idents: <#field_types>::init_state(world),)*
                    }
                }

                fn matches_component_set(state: &Self::State, _set_contains_id: &impl Fn(#path::component::ComponentId) -> bool) -> bool {
                    true #(&& <#field_types>::matches_component_set(&state.#named_field_idents, _set_contains_id))*
                }
            }
        };
        (item_struct, query_impl)
    };

    let (mutable_struct, mutable_impl) = impl_fetch(false);
    let (read_only_struct, read_only_impl) = if fetch_struct_attributes.is_mutable {
        let (readonly_state, read_only_impl) = impl_fetch(true);
        let read_only_structs = quote! {
            #[doc = "Automatically generated [`WorldQuery`] type for a read-only variant of [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`]."]
            #[automatically_derived]
            #visibility struct #read_only_struct_name #user_impl_generics #user_where_clauses {
                #(
                    #[doc = "Automatically generated read-only field for accessing `"]
                    #[doc = stringify!(#field_types)]
                    #[doc = "`."]
                    #field_visibilities #named_field_idents: #read_only_field_types,
                )*
            }

            #readonly_state
        };
        (read_only_structs, read_only_impl)
    } else {
        (quote! {}, quote! {})
    };

    let read_only_asserts = if fetch_struct_attributes.is_mutable {
        quote! {
            // Double-check that the data fetched by `<_ as WorldQuery>::ReadOnly` is read-only.
            // This is technically unnecessary as `<_ as WorldQuery>::ReadOnly: ReadOnlyWorldQuery`
            // but to protect against future mistakes we assert the assoc type implements `ReadOnlyWorldQuery` anyway
            #( assert_readonly::<#read_only_field_types>(); )*
        }
    } else {
        quote! {
            // Statically checks that the safety guarantee of `ReadOnlyWorldQuery` for `$fetch_struct_name` actually holds true.
            // We need this to make sure that we don't compile `ReadOnlyWorldQuery` if our struct contains nested `WorldQuery`
            // members that don't implement it. I.e.:
            // ```
            // #[derive(WorldQuery)]
            // pub struct Foo { a: &'static mut MyComponent }
            // ```
            #( assert_readonly::<#field_types>(); )*
        }
    };

    TokenStream::from(quote! {
        #mutable_struct

        #read_only_struct

        /// SAFETY: we assert fields are readonly below
        unsafe impl #user_impl_generics #path::query::ReadOnlyWorldQuery
            for #read_only_struct_name #user_ty_generics #user_where_clauses {}

        const _: () = {
            #[doc(hidden)]
            #[doc = "Automatically generated internal [`WorldQuery`] state type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], used for caching."]
            #[automatically_derived]
            #visibility struct #state_struct_name #user_impl_generics #user_where_clauses {
                #(#named_field_idents: <#field_types as #path::query::WorldQuery>::State,)*
            }

            #mutable_impl

            #read_only_impl
        };

        #[allow(dead_code)]
        const _: () = {
            fn assert_readonly<T>()
            where
                T: #path::query::ReadOnlyWorldQuery,
            {
            }

            // We generate a readonly assertion for every struct member.
            fn assert_all #user_impl_generics_with_world () #user_where_clauses_with_world {
                #read_only_asserts
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
                #(q.#field_idents;)*
                #(q2.#field_idents;)*
            }
        };
    })
}

struct WorldQueryFieldInfo {
    /// All field attributes except for `world_query` ones.
    attrs: Vec<Attribute>,
}

fn read_world_query_field_info(field: &Field) -> syn::Result<WorldQueryFieldInfo> {
    let mut attrs = Vec::new();
    for attr in &field.attrs {
        if attr
            .path()
            .get_ident()
            .map_or(false, |ident| ident == WORLD_QUERY_ATTRIBUTE_NAME)
        {
            return Err(syn::Error::new_spanned(
                attr,
                "#[derive(WorldQuery)] does not support field attributes.",
            ));
        }
        attrs.push(attr.clone());
    }

    Ok(WorldQueryFieldInfo { attrs })
}
