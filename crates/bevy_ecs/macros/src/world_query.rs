use proc_macro2::Ident;
use quote::quote;
use syn::{Attribute, Fields, ImplGenerics, TypeGenerics, Visibility, WhereClause};

#[allow(clippy::too_many_arguments)]
pub(crate) fn item_struct(
    path: &syn::Path,
    fields: &Fields,
    derive_macro_call: &proc_macro2::TokenStream,
    struct_name: &Ident,
    visibility: &Visibility,
    item_struct_name: &Ident,
    field_types: &Vec<proc_macro2::TokenStream>,
    user_impl_generics_with_world: &ImplGenerics,
    field_attrs: &Vec<Vec<Attribute>>,
    field_visibilities: &Vec<Visibility>,
    field_idents: &Vec<proc_macro2::TokenStream>,
    user_ty_generics: &TypeGenerics,
    user_ty_generics_with_world: &TypeGenerics,
    user_where_clauses_with_world: Option<&WhereClause>,
) -> proc_macro2::TokenStream {
    let item_attrs = quote!(
            #[doc = "Automatically generated [`WorldQuery`] item type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], returned when iterating over query results."]
            #[automatically_derived]
    );

    match fields {
        Fields::Named(_) => quote! {
            #derive_macro_call
            #item_attrs
            #visibility struct #item_struct_name #user_impl_generics_with_world #user_where_clauses_with_world {
                #(#(#field_attrs)* #field_visibilities #field_idents: <#field_types as #path::query::WorldQuery>::Item<'__w>,)*
            }
        },
        Fields::Unnamed(_) => quote! {
            #derive_macro_call
            #item_attrs
            #visibility struct #item_struct_name #user_impl_generics_with_world #user_where_clauses_with_world(
                #( #field_visibilities <#field_types as #path::query::WorldQuery>::Item<'__w>, )*
            );
        },
        Fields::Unit => quote! {
            #item_attrs
            #visibility type #item_struct_name #user_ty_generics_with_world = #struct_name #user_ty_generics;
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn world_query_impl(
    path: &syn::Path,
    struct_name: &Ident,
    visibility: &Visibility,
    item_struct_name: &Ident,
    fetch_struct_name: &Ident,
    field_types: &Vec<proc_macro2::TokenStream>,
    user_impl_generics: &ImplGenerics,
    user_impl_generics_with_world: &ImplGenerics,
    field_idents: &Vec<proc_macro2::TokenStream>,
    user_ty_generics: &TypeGenerics,
    user_ty_generics_with_world: &TypeGenerics,
    named_field_idents: &Vec<Ident>,
    marker_name: &Ident,
    state_struct_name: &Ident,
    user_where_clauses: Option<&WhereClause>,
    user_where_clauses_with_world: Option<&WhereClause>,
) -> proc_macro2::TokenStream {
    quote! {
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

            fn update_component_access(state: &Self::State, _access: &mut #path::query::FilteredAccess<#path::component::ComponentId>) {
                #( <#field_types>::update_component_access(&state.#named_field_idents, _access); )*
            }

            fn init_state(world: &mut #path::world::World) -> #state_struct_name #user_ty_generics {
                #state_struct_name {
                    #(#named_field_idents: <#field_types>::init_state(world),)*
                }
            }

            fn get_state(components: &#path::component::Components) -> Option<#state_struct_name #user_ty_generics> {
                Some(#state_struct_name {
                    #(#named_field_idents: <#field_types>::get_state(components)?,)*
                })
            }

            fn matches_component_set(state: &Self::State, _set_contains_id: &impl Fn(#path::component::ComponentId) -> bool) -> bool {
                true #(&& <#field_types>::matches_component_set(&state.#named_field_idents, _set_contains_id))*
            }
        }
    }
}
