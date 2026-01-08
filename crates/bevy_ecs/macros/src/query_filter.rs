use bevy_macro_utils::{ensure_no_collision, get_struct_fields};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, DeriveInput};

use crate::{bevy_ecs_path, world_query::world_query_impl};

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

    let fetch_struct_name = Ident::new(&format!("{struct_name}Fetch"), Span::call_site());
    let fetch_struct_name = ensure_no_collision(fetch_struct_name, tokens.clone());

    let marker_name =
        ensure_no_collision(format_ident!("_world_query_derive_marker"), tokens.clone());

    // Generate a name for the state struct that doesn't conflict
    // with the struct definition.
    let state_struct_name = Ident::new(&format!("{struct_name}State"), Span::call_site());
    let state_struct_name = ensure_no_collision(state_struct_name, tokens);

    let fields = match get_struct_fields(&ast.data, "derive(WorldQuery)") {
        Ok(fields) => fields,
        Err(e) => return e.into_compile_error().into(),
    };

    let field_members: Vec<_> = fields.members().collect();
    let field_aliases = fields
        .members()
        .map(|m| format_ident!("field{}", m))
        .collect();
    let field_types = fields.iter().map(|f| f.ty.clone()).collect();

    let world_query_impl = world_query_impl(
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

    let filter_impl = quote! {
        impl #user_impl_generics #path::query::QueryFilter for #struct_name #user_ty_generics #user_where_clauses {}
    };

    let filter_asserts = quote! {
        #( assert_filter::<#field_types>(); )*
    };

    TokenStream::from(quote! {
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
                #(q.#field_members;)*
                #(q2.#field_members;)*
            }
        };
    })
}
