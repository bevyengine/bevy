use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    Attribute, Data, DataStruct, DeriveInput, Field, Fields,
};

use crate::bevy_ecs_path;

#[derive(Default)]
struct FetchStructAttributes {
    pub is_mutable: bool,
    pub derive_args: Punctuated<syn::NestedMeta, syn::token::Comma>,
}

static MUTABLE_ATTRIBUTE_NAME: &str = "mutable";
static DERIVE_ATTRIBUTE_NAME: &str = "derive";

mod field_attr_keywords {
    syn::custom_keyword!(ignore);
}

pub static WORLD_QUERY_ATTRIBUTE_NAME: &str = "world_query";

pub fn derive_world_query_impl(ast: DeriveInput) -> TokenStream {
    let visibility = ast.vis;

    let mut fetch_struct_attributes = FetchStructAttributes::default();
    for attr in &ast.attrs {
        if !attr
            .path
            .get_ident()
            .map_or(false, |ident| ident == WORLD_QUERY_ATTRIBUTE_NAME)
        {
            continue;
        }

        attr.parse_args_with(|input: ParseStream| {
            let meta = input.parse_terminated::<syn::Meta, syn::token::Comma>(syn::Meta::parse)?;
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
                            "The `{}` attribute is expected to have no value or arguments",
                            MUTABLE_ATTRIBUTE_NAME
                        );
                    }
                } else if ident == DERIVE_ATTRIBUTE_NAME {
                    if let syn::Meta::List(meta_list) = meta {
                        fetch_struct_attributes
                            .derive_args
                            .extend(meta_list.nested.iter().cloned());
                    } else {
                        panic!(
                            "Expected a structured list within the `{}` attribute",
                            DERIVE_ATTRIBUTE_NAME
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
        let mut generics = ast.generics.clone();
        generics.params.insert(0, parse_quote!('__w));
        generics
    };
    let (user_impl_generics_with_world, user_ty_generics_with_world, user_where_clauses_with_world) =
        user_generics_with_world.split_for_impl();

    let struct_name = ast.ident.clone();
    let read_only_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnly"), Span::call_site())
    } else {
        struct_name.clone()
    };

    let item_struct_name = Ident::new(&format!("{struct_name}Item"), Span::call_site());
    let read_only_item_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnlyItem"), Span::call_site())
    } else {
        item_struct_name.clone()
    };

    let fetch_struct_name = Ident::new(&format!("{struct_name}Fetch"), Span::call_site());
    let read_only_fetch_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnlyFetch"), Span::call_site())
    } else {
        fetch_struct_name.clone()
    };

    let state_struct_name = Ident::new(&format!("{struct_name}State"), Span::call_site());

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields"),
    };

    let mut ignored_field_attrs = Vec::new();
    let mut ignored_field_visibilities = Vec::new();
    let mut ignored_field_idents = Vec::new();
    let mut ignored_field_types = Vec::new();
    let mut field_attrs = Vec::new();
    let mut field_visibilities = Vec::new();
    let mut field_idents = Vec::new();
    let mut field_types = Vec::new();
    let mut read_only_field_types = Vec::new();

    for field in fields {
        let WorldQueryFieldInfo { is_ignored, attrs } = read_world_query_field_info(field);

        let field_ident = field.ident.as_ref().unwrap().clone();
        if is_ignored {
            ignored_field_attrs.push(attrs);
            ignored_field_visibilities.push(field.vis.clone());
            ignored_field_idents.push(field_ident.clone());
            ignored_field_types.push(field.ty.clone());
        } else {
            field_attrs.push(attrs);
            field_visibilities.push(field.vis.clone());
            field_idents.push(field_ident.clone());
            let field_ty = field.ty.clone();
            field_types.push(quote!(#field_ty));
            read_only_field_types.push(quote!(<#field_ty as #path::query::WorldQuery>::ReadOnly));
        }
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

        quote! {
            #derive_macro_call
            #[doc = "Automatically generated [`WorldQuery`] item type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], returned when iterating over query results."]
            #[automatically_derived]
            #visibility struct #item_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                #(#(#field_attrs)* #field_visibilities #field_idents: <#field_types as #path::query::WorldQuery>::Item<'__w>,)*
                #(#(#ignored_field_attrs)* #ignored_field_visibilities #ignored_field_idents: #ignored_field_types,)*
            }

            #[doc(hidden)]
            #[doc = "Automatically generated internal [`WorldQuery`] fetch type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], used to define the world data accessed by this query."]
            #[automatically_derived]
            #visibility struct #fetch_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                #(#field_idents: <#field_types as #path::query::WorldQuery>::Fetch<'__w>,)*
                #(#ignored_field_idents: #ignored_field_types,)*
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
                        #(
                            #ignored_field_idents: item.#ignored_field_idents,
                        )*
                    }
                }

                unsafe fn init_fetch<'__w>(
                    _world: &'__w #path::world::World,
                    state: &Self::State,
                    _last_change_tick: u32,
                    _change_tick: u32
                ) -> <Self as #path::query::WorldQuery>::Fetch<'__w> {
                    #fetch_struct_name {
                        #(#field_idents:
                            <#field_types>::init_fetch(
                                _world,
                                &state.#field_idents,
                                _last_change_tick,
                                _change_tick
                            ),
                        )*
                        #(#ignored_field_idents: Default::default(),)*
                    }
                }

                unsafe fn clone_fetch<'__w>(
                    _fetch: &<Self as #path::query::WorldQuery>::Fetch<'__w>
                ) -> <Self as #path::query::WorldQuery>::Fetch<'__w> {
                    #fetch_struct_name {
                        #(
                            #field_idents: <#field_types>::clone_fetch(& _fetch. #field_idents),
                        )*
                        #(
                            #ignored_field_idents: Default::default(),
                        )*
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
                    #(<#field_types>::set_archetype(&mut _fetch.#field_idents, &_state.#field_idents, _archetype, _table);)*
                }

                /// SAFETY: we call `set_table` for each member that implements `Fetch`
                #[inline]
                unsafe fn set_table<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _state: &Self::State,
                    _table: &'__w #path::storage::Table
                ) {
                    #(<#field_types>::set_table(&mut _fetch.#field_idents, &_state.#field_idents, _table);)*
                }

                /// SAFETY: we call `fetch` for each member that implements `Fetch`.
                #[inline(always)]
                unsafe fn fetch<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _entity: #path::entity::Entity,
                    _table_row: usize
                ) -> <Self as #path::query::WorldQuery>::Item<'__w> {
                    Self::Item {
                        #(#field_idents: <#field_types>::fetch(&mut _fetch.#field_idents, _entity, _table_row),)*
                        #(#ignored_field_idents: Default::default(),)*
                    }
                }

                #[allow(unused_variables)]
                #[inline(always)]
                unsafe fn filter_fetch<'__w>(
                    _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                    _entity: #path::entity::Entity,
                    _table_row: usize
                ) -> bool {
                    true #(&& <#field_types>::filter_fetch(&mut _fetch.#field_idents, _entity, _table_row))*
                }

                fn update_component_access(state: &Self::State, _access: &mut #path::query::FilteredAccess<#path::component::ComponentId>) {
                    #( <#field_types>::update_component_access(&state.#field_idents, _access); )*
                }

                fn update_archetype_component_access(
                    state: &Self::State,
                    _archetype: &#path::archetype::Archetype,
                    _access: &mut #path::query::Access<#path::archetype::ArchetypeComponentId>
                ) {
                    #(
                        <#field_types>::update_archetype_component_access(&state.#field_idents, _archetype, _access);
                    )*
                }

                fn init_state(world: &mut #path::world::World) -> #state_struct_name #user_ty_generics {
                    #state_struct_name {
                        #(#field_idents: <#field_types>::init_state(world),)*
                        #(#ignored_field_idents: Default::default(),)*
                    }
                }

                fn matches_component_set(state: &Self::State, _set_contains_id: &impl Fn(#path::component::ComponentId) -> bool) -> bool {
                    true #(&& <#field_types>::matches_component_set(&state.#field_idents, _set_contains_id))*
                }
            }
        }
    };

    let mutable_impl = impl_fetch(false);
    let readonly_impl = if fetch_struct_attributes.is_mutable {
        let world_query_impl = impl_fetch(true);
        quote! {
            #[doc(hidden)]
            #[doc = "Automatically generated internal [`WorldQuery`] type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], used for read-only access."]
            #[automatically_derived]
            #visibility struct #read_only_struct_name #user_impl_generics #user_where_clauses {
                #( #field_idents: #read_only_field_types, )*
                #(#(#ignored_field_attrs)* #ignored_field_visibilities #ignored_field_idents: #ignored_field_types,)*
            }

            #world_query_impl
        }
    } else {
        quote! {}
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
        #mutable_impl

        #readonly_impl

        #[doc(hidden)]
        #[doc = "Automatically generated internal [`WorldQuery`] state type for [`"]
        #[doc = stringify!(#struct_name)]
        #[doc = "`], used for caching."]
        #[automatically_derived]
        #visibility struct #state_struct_name #user_impl_generics #user_where_clauses {
            #(#field_idents: <#field_types as #path::query::WorldQuery>::State,)*
            #(#ignored_field_idents: #ignored_field_types,)*
        }

        /// SAFETY: we assert fields are readonly below
        unsafe impl #user_impl_generics #path::query::ReadOnlyWorldQuery
            for #read_only_struct_name #user_ty_generics #user_where_clauses {}

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
                #(q.#ignored_field_idents;)*
                #(q2.#field_idents;)*
                #(q2.#ignored_field_idents;)*

            }
        };
    })
}

struct WorldQueryFieldInfo {
    /// Has `#[fetch(ignore)]` or `#[filter_fetch(ignore)]` attribute.
    is_ignored: bool,
    /// All field attributes except for `world_query` ones.
    attrs: Vec<Attribute>,
}

fn read_world_query_field_info(field: &Field) -> WorldQueryFieldInfo {
    let is_ignored = field
        .attrs
        .iter()
        .find(|attr| {
            attr.path
                .get_ident()
                .map_or(false, |ident| ident == WORLD_QUERY_ATTRIBUTE_NAME)
        })
        .map_or(false, |attr| {
            let mut is_ignored = false;
            attr.parse_args_with(|input: ParseStream| {
                if input
                    .parse::<Option<field_attr_keywords::ignore>>()?
                    .is_some()
                {
                    is_ignored = true;
                }
                Ok(())
            })
            .unwrap_or_else(|_| panic!("Invalid `{WORLD_QUERY_ATTRIBUTE_NAME}` attribute format"));

            is_ignored
        });

    let attrs = field
        .attrs
        .iter()
        .filter(|attr| {
            attr.path
                .get_ident()
                .map_or(true, |ident| ident != WORLD_QUERY_ATTRIBUTE_NAME)
        })
        .cloned()
        .collect();

    WorldQueryFieldInfo { is_ignored, attrs }
}
