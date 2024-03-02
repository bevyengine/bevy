use bevy_macro_utils::ensure_no_collision;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Index};

use crate::{
    bevy_ecs_path,
    world_query::{item_struct, world_query_impl},
};

mod field_attr_keywords {
    syn::custom_keyword!(ignore);
}

pub fn derive_query_filter_impl(input: TokenStream) -> TokenStream {
    let tokens = input.clone();

    let ast = parse_macro_input!(input as DeriveInput);
    let visibility = ast.vis;

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

    let item_struct_name = Ident::new(&format!("{struct_name}Item"), Span::call_site());

    let fetch_struct_name = Ident::new(&format!("{struct_name}Fetch"), Span::call_site());
    let fetch_struct_name = ensure_no_collision(fetch_struct_name, tokens.clone());

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
    for (i, field) in fields.iter().enumerate() {
        let attrs = field.attrs.clone();

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
    }

    let derive_macro_call = quote!();

    let item_struct = item_struct(
        &path,
        fields,
        &derive_macro_call,
        &struct_name,
        &visibility,
        &item_struct_name,
        &field_types,
        &user_impl_generics_with_world,
        &field_attrs,
        &field_visibilities,
        &field_idents,
        &user_ty_generics,
        &user_ty_generics_with_world,
        user_where_clauses_with_world,
    );

    let world_query_impl = world_query_impl(
        &path,
        &struct_name,
        &visibility,
        &item_struct_name,
        &fetch_struct_name,
        &field_types,
        &user_impl_generics,
        &user_impl_generics_with_world,
        &field_idents,
        &user_ty_generics,
        &user_ty_generics_with_world,
        &named_field_idents,
        &marker_name,
        &state_struct_name,
        user_where_clauses,
        user_where_clauses_with_world,
    );

    let filter_impl = quote! {
        impl #user_impl_generics #path::query::QueryFilter
        for #struct_name #user_ty_generics #user_where_clauses {
            const IS_ARCHETYPAL: bool = true #(&& <#field_types>::IS_ARCHETYPAL)*;

            #[allow(unused_variables)]
            #[inline(always)]
            unsafe fn filter_fetch<'__w>(
                _fetch: &mut <Self as #path::query::WorldQuery>::Fetch<'__w>,
                _entity: #path::entity::Entity,
                _table_row: #path::storage::TableRow,
            ) -> bool {
                true #(&& <#field_types>::filter_fetch(&mut _fetch.#named_field_idents, _entity, _table_row))*
            }
        }
    };

    let filter_asserts = quote! {
        #( assert_filter::<#field_types>(); )*
    };

    TokenStream::from(quote! {
        #item_struct

        const _: () = {
            #[doc(hidden)]
            #[doc = "Automatically generated internal [`WorldQuery`] state type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], used for caching."]
            #[automatically_derived]
            #visibility struct #state_struct_name #user_impl_generics #user_where_clauses {
                #(#named_field_idents: <#field_types as #path::query::WorldQuery>::State,)*
            }

            #world_query_impl

            #filter_impl
        };

        #[allow(dead_code)]
        const _: () = {

            fn assert_filter<T>()
            where
                T: #path::query::QueryFilter,
            {
            }

            // We generate a filter assertion for every struct member.
            fn assert_all #user_impl_generics_with_world () #user_where_clauses_with_world {
                #filter_asserts
            }
        };

        // The original struct will most likely be left unused. As we don't want our users having
        // to specify `#[allow(dead_code)]` for their custom queries, we are using this cursed
        // workaround.
        #[allow(dead_code)]
        const _: () = {
            fn dead_code_workaround #user_impl_generics (
                q: #struct_name #user_ty_generics,
                q2: #struct_name #user_ty_generics
            ) #user_where_clauses {
                #(q.#field_idents;)*
                #(q2.#field_idents;)*
            }
        };
    })
}
