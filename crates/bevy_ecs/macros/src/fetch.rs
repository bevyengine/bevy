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
        .unwrap_or_else(|_| panic!("Invalid `{}` attribute format", WORLD_QUERY_ATTRIBUTE_NAME));
    }

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
        Ident::new(&format!("{}ReadOnly", struct_name), Span::call_site())
    } else {
        struct_name.clone()
    };

    let item_struct_name = Ident::new(&format!("{}Item", struct_name), Span::call_site());
    let read_only_item_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{}ReadOnlyItem", struct_name), Span::call_site())
    } else {
        item_struct_name.clone()
    };

    let fetch_struct_name = Ident::new(&format!("{}Fetch", struct_name), Span::call_site());
    let read_only_fetch_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{}ReadOnlyFetch", struct_name), Span::call_site())
    } else {
        fetch_struct_name.clone()
    };

    let state_struct_name = Ident::new(&format!("{}State", struct_name), Span::call_site());

    let fetch_type_alias = Ident::new("QueryFetch", Span::call_site());
    let read_only_fetch_type_alias = Ident::new("ROQueryFetch", Span::call_site());
    let item_type_alias = Ident::new("QueryItem", Span::call_site());
    let read_only_item_type_alias = Ident::new("ROQueryItem", Span::call_site());

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
            field_types.push(field.ty.clone());
        }
    }

    let derive_args = &fetch_struct_attributes.derive_args;
    // `#[derive()]` is valid syntax
    let derive_macro_call = quote! { #[derive(#derive_args)] };

    let path = bevy_ecs_path();

    let impl_fetch = |is_readonly: bool, fetch_struct_name: Ident, item_struct_name: Ident| {
        let fetch_type_alias = if is_readonly {
            &read_only_fetch_type_alias
        } else {
            &fetch_type_alias
        };
        let item_type_alias = if is_readonly {
            &read_only_item_type_alias
        } else {
            &item_type_alias
        };

        quote! {
            #derive_macro_call
            #[automatically_derived]
            #visibility struct #item_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                #(#(#field_attrs)* #field_visibilities #field_idents: #path::query::#item_type_alias<'__w, #field_types>,)*
                #(#(#ignored_field_attrs)* #ignored_field_visibilities #ignored_field_idents: #ignored_field_types,)*
            }

            #[derive(Clone)]
            #[doc(hidden)]
            #visibility struct #fetch_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                #(#field_idents: #path::query::#fetch_type_alias::<'__w, #field_types>,)*
                #(#ignored_field_idents: #ignored_field_types,)*
            }

            // SAFETY: `update_component_access` and `update_archetype_component_access` are called on every field
            unsafe impl #user_impl_generics_with_world #path::query::Fetch<'__w>
                for #fetch_struct_name #user_ty_generics_with_world #user_where_clauses_with_world {

                type Item = #item_struct_name #user_ty_generics_with_world;
                type State = #state_struct_name #user_ty_generics;

                unsafe fn init(_world: &'__w #path::world::World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                    Self {
                        #(#field_idents:
                            #path::query::#fetch_type_alias::<'__w, #field_types>::init(
                                _world,
                                &state.#field_idents,
                                _last_change_tick,
                                _change_tick
                            ),
                        )*
                        #(#ignored_field_idents: Default::default(),)*
                    }
                }

                const IS_DENSE: bool = true #(&& #path::query::#fetch_type_alias::<'__w, #field_types>::IS_DENSE)*;

                const IS_ARCHETYPAL: bool = true #(&& #path::query::#fetch_type_alias::<'__w, #field_types>::IS_ARCHETYPAL)*;

                /// SAFETY: we call `set_archetype` for each member that implements `Fetch`
                #[inline]
                unsafe fn set_archetype(
                    &mut self,
                    _state: &Self::State,
                    _archetype: &'__w #path::archetype::Archetype,
                    _tables: &'__w #path::storage::Tables
                ) {
                    #(self.#field_idents.set_archetype(&_state.#field_idents, _archetype, _tables);)*
                }

                /// SAFETY: we call `set_table` for each member that implements `Fetch`
                #[inline]
                unsafe fn set_table(&mut self, _state: &Self::State, _table: &'__w #path::storage::Table) {
                    #(self.#field_idents.set_table(&_state.#field_idents, _table);)*
                }

                /// SAFETY: we call `table_fetch` for each member that implements `Fetch`.
                #[inline]
                unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {
                    Self::Item {
                        #(#field_idents: self.#field_idents.table_fetch(_table_row),)*
                        #(#ignored_field_idents: Default::default(),)*
                    }
                }

                /// SAFETY: we call `archetype_fetch` for each member that implements `Fetch`.
                #[inline]
                unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                    Self::Item {
                        #(#field_idents: self.#field_idents.archetype_fetch(_archetype_index),)*
                        #(#ignored_field_idents: Default::default(),)*
                    }
                }

                #[allow(unused_variables)]
                #[inline]
                unsafe fn table_filter_fetch(&mut self, _table_row: usize) -> bool {
                    true #(&& self.#field_idents.table_filter_fetch(_table_row))*
                }

                #[allow(unused_variables)]
                #[inline]
                unsafe fn archetype_filter_fetch(&mut self, _archetype_index: usize) -> bool {
                    true #(&& self.#field_idents.archetype_filter_fetch(_archetype_index))*
                }

                fn update_component_access(state: &Self::State, _access: &mut #path::query::FilteredAccess<#path::component::ComponentId>) {
                    #( #path::query::#fetch_type_alias::<'static, #field_types> :: update_component_access(&state.#field_idents, _access);  )*
                }

                fn update_archetype_component_access(state: &Self::State, _archetype: &#path::archetype::Archetype, _access: &mut #path::query::Access<#path::archetype::ArchetypeComponentId>) {
                    #(
                        #path::query::#fetch_type_alias::<'static, #field_types>
                            :: update_archetype_component_access(&state.#field_idents, _archetype, _access);
                    )*
                }
            }
        }
    };

    let fetch_impl = impl_fetch(false, fetch_struct_name.clone(), item_struct_name.clone());

    let state_impl = quote! {
        #[doc(hidden)]
        #visibility struct #state_struct_name #user_impl_generics #user_where_clauses {

            #(#field_idents: <#field_types as #path::query::WorldQuery>::State,)*
            #(#ignored_field_idents: #ignored_field_types,)*
        }

        impl #user_impl_generics #path::query::FetchState for #state_struct_name #user_ty_generics #user_where_clauses {
            fn init(world: &mut #path::world::World) -> Self {
                #state_struct_name {
                    #(#field_idents: <<#field_types as #path::query::WorldQuery>::State as #path::query::FetchState>::init(world),)*
                    #(#ignored_field_idents: Default::default(),)*
                }
            }

            fn matches_component_set(&self, _set_contains_id: &impl Fn(#path::component::ComponentId) -> bool) -> bool {
                true #(&& self.#field_idents.matches_component_set(_set_contains_id))*

            }
        }
    };

    let read_only_fetch_impl = if fetch_struct_attributes.is_mutable {
        impl_fetch(
            true,
            read_only_fetch_struct_name.clone(),
            read_only_item_struct_name.clone(),
        )
    } else {
        quote! {}
    };

    let read_only_world_query_impl = if fetch_struct_attributes.is_mutable {
        quote! {
            #[automatically_derived]
            #visibility struct #read_only_struct_name #user_impl_generics #user_where_clauses {
                #( #field_idents: < #field_types as #path::query::WorldQuery >::ReadOnly, )*
                #(#(#ignored_field_attrs)* #ignored_field_visibilities #ignored_field_idents: #ignored_field_types,)*
            }

            // SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
            unsafe impl #user_impl_generics #path::query::WorldQuery for #read_only_struct_name #user_ty_generics #user_where_clauses {
                type ReadOnly = Self;
                type State = #state_struct_name #user_ty_generics;

                fn shrink<'__wlong: '__wshort, '__wshort>(item: #path::query::#item_type_alias<'__wlong, Self>)
                -> #path::query::#item_type_alias<'__wshort, Self> {
                    #read_only_item_struct_name {
                        #(
                            #field_idents : <
                                < #field_types as #path::query::WorldQuery >::ReadOnly as #path::query::WorldQuery
                            > :: shrink( item.#field_idents ),
                        )*
                        #(
                            #ignored_field_idents: item.#ignored_field_idents,
                        )*
                    }
                }
            }

            impl #user_impl_generics_with_world #path::query::WorldQueryGats<'__w> for #read_only_struct_name #user_ty_generics #user_where_clauses {
                type Fetch = #read_only_fetch_struct_name #user_ty_generics_with_world;
                type _State = #state_struct_name #user_ty_generics;
            }
        }
    } else {
        quote! {}
    };

    let read_only_asserts = if fetch_struct_attributes.is_mutable {
        quote! {
            // Double-check that the data fetched by `<_ as WorldQuery>::ReadOnly` is read-only.
            // This is technically unnecessary as `<_ as WorldQuery>::ReadOnly: ReadOnlyWorldQuery`
            // but to protect against future mistakes we assert the assoc type implements `ReadOnlyWorldQuery` anyway
            #( assert_readonly::< < #field_types as #path::query::WorldQuery > :: ReadOnly >(); )*
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
        #fetch_impl

        #state_impl

        #read_only_fetch_impl

        #read_only_world_query_impl

        // SAFETY: if the worldquery is mutable this defers to soundness of the `#field_types: WorldQuery` impl, otherwise
        // if the world query is immutable then `#read_only_struct_name #user_ty_generics` is the same type as `#struct_name #user_ty_generics`
        unsafe impl #user_impl_generics #path::query::WorldQuery for #struct_name #user_ty_generics #user_where_clauses {
            type ReadOnly = #read_only_struct_name #user_ty_generics;
            type State = #state_struct_name #user_ty_generics;
            fn shrink<'__wlong: '__wshort, '__wshort>(item: #path::query::#item_type_alias<'__wlong, Self>)
                -> #path::query::#item_type_alias<'__wshort, Self> {
                    #item_struct_name {
                        #(
                           #field_idents : < #field_types as #path::query::WorldQuery> :: shrink( item.#field_idents ),
                        )*
                        #(
                            #ignored_field_idents: item.#ignored_field_idents,
                        )*
                    }
                }
        }

        impl #user_impl_generics_with_world #path::query::WorldQueryGats<'__w> for #struct_name #user_ty_generics #user_where_clauses {
            type Fetch = #fetch_struct_name #user_ty_generics_with_world;
            type _State = #state_struct_name #user_ty_generics;
        }

        /// SAFETY: each item in the struct is read only
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
            fn dead_code_workaround #user_impl_generics (q: #struct_name #user_ty_generics) #user_where_clauses {
                #(q.#field_idents;)*
                #(q.#ignored_field_idents;)*
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
            .unwrap_or_else(|_| {
                panic!("Invalid `{}` attribute format", WORLD_QUERY_ATTRIBUTE_NAME)
            });

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
