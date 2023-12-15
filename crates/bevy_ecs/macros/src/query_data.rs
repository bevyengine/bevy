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

use crate::{
    bevy_ecs_path,
    world_query::{item_struct, world_query_impl},
};

#[derive(Default)]
struct QueryDataAttributes {
    pub is_mutable: bool,

    pub derive_args: Punctuated<Meta, syn::token::Comma>,
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
        if !attr
            .path()
            .get_ident()
            .map_or(false, |ident| ident == QUERY_DATA_ATTRIBUTE_NAME)
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
                    if let Meta::Path(_) = meta {
                        attributes.is_mutable = true;
                    } else {
                        panic!(
                            "The `{MUTABLE_ATTRIBUTE_NAME}` attribute is expected to have no value or arguments",
                        );
                    }
                }
                else if ident == DERIVE_ATTRIBUTE_NAME {
                    if let Meta::List(meta_list) = meta {
                        meta_list.parse_nested_meta(|meta| {
                            attributes.derive_args.push(Meta::Path(meta.path));
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
        .unwrap_or_else(|_| panic!("Invalid `{QUERY_DATA_ATTRIBUTE_NAME}` attribute format"));
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
    let read_only_struct_name = if attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnly"), Span::call_site())
    } else {
        #[allow(clippy::redundant_clone)]
        struct_name.clone()
    };

    let item_struct_name = Ident::new(&format!("{struct_name}Item"), Span::call_site());
    let read_only_item_struct_name = if attributes.is_mutable {
        Ident::new(&format!("{struct_name}ReadOnlyItem"), Span::call_site())
    } else {
        #[allow(clippy::redundant_clone)]
        item_struct_name.clone()
    };

    let fetch_struct_name = Ident::new(&format!("{struct_name}Fetch"), Span::call_site());
    let fetch_struct_name = ensure_no_collision(fetch_struct_name, tokens.clone());
    let read_only_fetch_struct_name = if attributes.is_mutable {
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
            "#[derive(QueryData)]` only supports structs",
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
            Ok(QueryDataFieldInfo { attrs }) => attrs,
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
        read_only_field_types.push(quote!(<#field_ty as #path::query::QueryData>::ReadOnly));
    }

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
        &user_impl_generics_with_world,
        &field_attrs,
        &field_visibilities,
        &field_idents,
        &user_ty_generics,
        &user_ty_generics_with_world,
        user_where_clauses_with_world,
    );
    let mutable_world_query_impl = world_query_impl(
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
            &user_impl_generics_with_world,
            &field_attrs,
            &field_visibilities,
            &field_idents,
            &user_ty_generics,
            &user_ty_generics_with_world,
            user_where_clauses_with_world,
        );
        let readonly_world_query_impl = world_query_impl(
            &path,
            &read_only_struct_name,
            &visibility,
            &read_only_item_struct_name,
            &read_only_fetch_struct_name,
            &read_only_field_types,
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
                    type ReadOnly = #read_only_struct_name #user_ty_generics;
                }
            }
        } else {
            quote! {}
        };

        quote! {
            /// SAFETY: we assert fields are readonly below
            unsafe impl #user_impl_generics #path::query::QueryData
            for #struct_name #user_ty_generics #user_where_clauses {
                type ReadOnly = #read_only_struct_name #user_ty_generics;
            }

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
            #[doc = "Automatically generated internal [`WorldQuery`] state type for [`"]
            #[doc = stringify!(#struct_name)]
            #[doc = "`], used for caching."]
            #[automatically_derived]
            #visibility struct #state_struct_name #user_impl_generics #user_where_clauses {
                #(#named_field_idents: <#field_types as #path::query::WorldQuery>::State,)*
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
                #(q.#field_idents;)*
                #(q2.#field_idents;)*
            }
        };
    })
}

struct QueryDataFieldInfo {
    /// All field attributes except for `query_data` ones.
    attrs: Vec<Attribute>,
}

fn read_world_query_field_info(field: &Field) -> syn::Result<QueryDataFieldInfo> {
    let mut attrs = Vec::new();
    for attr in &field.attrs {
        if attr
            .path()
            .get_ident()
            .map_or(false, |ident| ident == QUERY_DATA_ATTRIBUTE_NAME)
        {
            return Err(syn::Error::new_spanned(
                attr,
                "#[derive(QueryData)] does not support field attributes.",
            ));
        }
        attrs.push(attr.clone());
    }

    Ok(QueryDataFieldInfo { attrs })
}
