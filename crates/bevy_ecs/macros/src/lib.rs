// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// modified by Bevy contributors

extern crate proc_macro;

use std::borrow::Cow;

use find_crate::Manifest;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::ParseStream, parse_macro_input, Data, DataStruct, DeriveInput, Error, Field, Fields,
    Ident, Index, Lifetime, Path, Result,
};

/// Implement `Bundle` for a monomorphic struct
///
/// Using derived `Bundle` impls improves spawn performance and can be convenient when combined with
/// other derives like `serde::Deserialize`.
#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_bundle_(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
    .into()
}

#[allow(clippy::cognitive_complexity)]
fn derive_bundle_(input: DeriveInput) -> Result<TokenStream2> {
    let ident = input.ident;
    let data = match input.data {
        syn::Data::Struct(s) => s,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "derive(Bundle) does not support enums or unions",
            ))
        }
    };
    let (tys, field_members) = struct_fields(&data.fields);
    let manifest = Manifest::new().unwrap();
    let path_str = if let Some(package) = manifest.find(|name| name == "bevy") {
        format!("{}::ecs", package.name)
    } else if let Some(package) = manifest.find(|name| name == "bevy_internal") {
        format!("{}::ecs", package.name)
    } else if let Some(package) = manifest.find(|name| name == "bevy_ecs") {
        package.name
    } else {
        "bevy_ecs".to_string()
    };
    let crate_path: Path = syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap();
    let field_idents = member_as_idents(&field_members);
    let generics = add_additional_bounds_to_generic_params(&crate_path, input.generics);

    let dyn_bundle_code =
        gen_dynamic_bundle_impl(&crate_path, &ident, &generics, &field_members, &tys);
    let bundle_code = if tys.is_empty() {
        gen_unit_struct_bundle_impl(&crate_path, ident, &generics)
    } else {
        gen_bundle_impl(
            &crate_path,
            &ident,
            &generics,
            &field_members,
            &field_idents,
            &tys,
        )
    };
    let mut ts = dyn_bundle_code;
    ts.extend(bundle_code);
    Ok(ts)
}

fn gen_dynamic_bundle_impl(
    crate_path: &syn::Path,
    ident: &syn::Ident,
    generics: &syn::Generics,
    field_members: &[syn::Member],
    tys: &[&syn::Type],
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::#crate_path::DynamicBundle for #ident #ty_generics #where_clause {
            fn with_ids<__hecs__T>(&self, f: impl ::std::ops::FnOnce(&[::std::any::TypeId]) -> __hecs__T) -> __hecs__T {
                <Self as ::#crate_path::Bundle>::with_static_ids(f)
            }

            fn type_info(&self) -> ::std::vec::Vec<::#crate_path::TypeInfo> {
                <Self as ::#crate_path::Bundle>::static_type_info()
            }

            #[allow(clippy::forget_copy)]
            unsafe fn put(mut self, mut f: impl ::std::ops::FnMut(*mut u8, ::std::any::TypeId, usize) -> bool) {
                #(
                    if f((&mut self.#field_members as *mut #tys).cast::<u8>(), ::std::any::TypeId::of::<#tys>(), ::std::mem::size_of::<#tys>()) {
                        #[allow(clippy::forget_copy)]
                        ::std::mem::forget(self.#field_members);
                    }
                )*
            }
        }
    }
}

fn gen_bundle_impl(
    crate_path: &syn::Path,
    ident: &syn::Ident,
    generics: &syn::Generics,
    field_members: &[syn::Member],
    field_idents: &[Cow<syn::Ident>],
    tys: &[&syn::Type],
) -> TokenStream2 {
    let num_tys = tys.len();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let with_static_ids_inner = quote! {
        {
            let mut tys = [#((::std::mem::align_of::<#tys>(), ::std::any::TypeId::of::<#tys>())),*];
            tys.sort_unstable_by(|x, y| {
                ::std::cmp::Ord::cmp(&x.0, &y.0)
                    .reverse()
                    .then(::std::cmp::Ord::cmp(&x.1, &y.1))
            });
            let mut ids = [::std::any::TypeId::of::<()>(); #num_tys];
            for (id, info) in ::std::iter::Iterator::zip(ids.iter_mut(), tys.iter()) {
                *id = info.1;
            }
            ids
        }
    };
    let with_static_ids_body = if generics.params.is_empty() {
        quote! {
            ::#crate_path::lazy_static::lazy_static! {
                static ref ELEMENTS: [::std::any::TypeId; #num_tys] = {
                    #with_static_ids_inner
                };
            }
            f(&*ELEMENTS)
        }
    } else {
        quote! {
            f(&#with_static_ids_inner)
        }
    };
    quote! {
        impl #impl_generics ::#crate_path::Bundle for #ident #ty_generics #where_clause {
            #[allow(non_camel_case_types)]
            fn with_static_ids<__hecs__T>(f: impl ::std::ops::FnOnce(&[::std::any::TypeId]) -> __hecs__T) -> __hecs__T {
                #with_static_ids_body
            }

            fn static_type_info() -> ::std::vec::Vec<::#crate_path::TypeInfo> {
                let mut info = ::std::vec![#(::#crate_path::TypeInfo::of::<#tys>()),*];
                info.sort_unstable();
                info
            }

            unsafe fn get(
                mut f: impl ::std::ops::FnMut(::std::any::TypeId, usize) -> ::std::option::Option<::std::ptr::NonNull<u8>>,
            ) -> ::std::result::Result<Self, ::#crate_path::MissingComponent> {
                #(
                    let #field_idents = f(::std::any::TypeId::of::<#tys>(), ::std::mem::size_of::<#tys>())
                            .ok_or_else(::#crate_path::MissingComponent::new::<#tys>)?
                            .cast::<#tys>()
                            .as_ptr();
                )*
                ::std::result::Result::Ok(Self { #( #field_members: #field_idents.read(), )* })
            }
        }
    }
}

// no reason to generate a static for unit structs
fn gen_unit_struct_bundle_impl(
    crate_path: &syn::Path,
    ident: syn::Ident,
    generics: &syn::Generics,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::#crate_path::Bundle for #ident #ty_generics #where_clause {
            #[allow(non_camel_case_types)]
            fn with_static_ids<__hecs__T>(f: impl ::std::ops::FnOnce(&[::std::any::TypeId]) -> __hecs__T) -> __hecs__T { f(&[]) }
            fn static_type_info() -> ::std::vec::Vec<::#crate_path::TypeInfo> { ::std::vec::Vec::new() }

            unsafe fn get(
                f: impl ::std::ops::FnMut(::std::any::TypeId, usize) -> ::std::option::Option<::std::ptr::NonNull<u8>>,
            ) -> Result<Self, ::#crate_path::MissingComponent> {
                Ok(Self {/* for some reason this works for all unit struct variations */})
            }
        }
    }
}

fn make_component_trait_bound(crate_path: &syn::Path) -> syn::TraitBound {
    syn::TraitBound {
        paren_token: None,
        modifier: syn::TraitBoundModifier::None,
        lifetimes: None,
        path: syn::parse_quote!(::#crate_path::Component),
    }
}

fn add_additional_bounds_to_generic_params(
    crate_path: &syn::Path,
    mut generics: syn::Generics,
) -> syn::Generics {
    generics.type_params_mut().for_each(|tp| {
        tp.bounds
            .push(syn::TypeParamBound::Trait(make_component_trait_bound(
                crate_path,
            )))
    });
    generics
}

fn struct_fields(fields: &syn::Fields) -> (Vec<&syn::Type>, Vec<syn::Member>) {
    match fields {
        syn::Fields::Named(ref fields) => fields
            .named
            .iter()
            .map(|f| (&f.ty, syn::Member::Named(f.ident.clone().unwrap())))
            .unzip(),
        syn::Fields::Unnamed(ref fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    &f.ty,
                    syn::Member::Unnamed(syn::Index {
                        index: i as u32,
                        span: Span::call_site(),
                    }),
                )
            })
            .unzip(),
        syn::Fields::Unit => (Vec::new(), Vec::new()),
    }
}

fn member_as_idents(members: &[syn::Member]) -> Vec<Cow<'_, syn::Ident>> {
    members
        .iter()
        .map(|member| match member {
            syn::Member::Named(ident) => Cow::Borrowed(ident),
            &syn::Member::Unnamed(syn::Index { index, span }) => {
                Cow::Owned(syn::Ident::new(&format!("tuple_field_{}", index), span))
            }
        })
        .collect()
}

fn get_idents(fmt_string: fn(usize) -> String, count: usize) -> Vec<Ident> {
    (0..count)
        .map(|i| Ident::new(&fmt_string(i), Span::call_site()))
        .collect::<Vec<Ident>>()
}

fn get_lifetimes(fmt_string: fn(usize) -> String, count: usize) -> Vec<Lifetime> {
    (0..count)
        .map(|i| Lifetime::new(&fmt_string(i), Span::call_site()))
        .collect::<Vec<Lifetime>>()
}

#[proc_macro]
pub fn impl_query_set(_input: TokenStream) -> TokenStream {
    let mut tokens = TokenStream::new();
    let max_queries = 4;
    let queries = get_idents(|i| format!("Q{}", i), max_queries);
    let filters = get_idents(|i| format!("F{}", i), max_queries);
    let lifetimes = get_lifetimes(|i| format!("'q{}", i), max_queries);
    let mut query_fns = Vec::new();
    let mut query_fn_muts = Vec::new();
    for i in 0..max_queries {
        let query = &queries[i];
        let lifetime = &lifetimes[i];
        let filter = &filters[i];
        let fn_name = Ident::new(&format!("q{}", i), Span::call_site());
        let fn_name_mut = Ident::new(&format!("q{}_mut", i), Span::call_site());
        let index = Index::from(i);
        query_fns.push(quote! {
            pub fn #fn_name(&self) -> &Query<#lifetime, #query, #filter> {
                &self.value.#index
            }
        });
        query_fn_muts.push(quote! {
            pub fn #fn_name_mut(&mut self) -> &mut Query<#lifetime, #query, #filter> {
                &mut self.value.#index
            }
        });
    }

    for query_count in 1..=max_queries {
        let query = &queries[0..query_count];
        let filter = &filters[0..query_count];
        let lifetime = &lifetimes[0..query_count];
        let query_fn = &query_fns[0..query_count];
        let query_fn_mut = &query_fn_muts[0..query_count];
        tokens.extend(TokenStream::from(quote! {
            impl<#(#lifetime,)* #(#query: WorldQuery,)* #(#filter: QueryFilter,)*> QueryTuple for (#(Query<#lifetime, #query, #filter>,)*) {
                unsafe fn new(world: &World, component_access: &TypeAccess<ArchetypeComponent>) -> Self {
                    (
                        #(
                            Query::<#query, #filter>::new(
                                std::mem::transmute(world),
                                std::mem::transmute(component_access),
                            ),
                        )*
                    )
                }

                fn get_accesses() -> Vec<QueryAccess> {
                    vec![
                        #(QueryAccess::union(vec![<#query::Fetch as Fetch>::access(), #filter::access()]),)*
                    ]
                }
            }

            impl<#(#lifetime,)* #(#query: WorldQuery,)* #(#filter: QueryFilter,)*> QuerySet<(#(Query<#lifetime, #query, #filter>,)*)> {
                #(#query_fn)*
                #(#query_fn_mut)*
            }
        }));
    }

    tokens
}

#[derive(Default)]
struct SystemParamFieldAttributes {
    pub ignore: bool,
}

static SYSTEM_PARAM_ATTRIBUTE_NAME: &str = "system_param";

#[proc_macro_derive(SystemParam, attributes(system_param))]
pub fn derive_system_param(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    let manifest = Manifest::new().unwrap();
    let path_str = if let Some(package) = manifest.find(|name| name == "bevy") {
        format!("{}::ecs", package.name)
    } else {
        "bevy_ecs".to_string()
    };
    let path: Path = syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap();

    let field_attributes = fields
        .iter()
        .map(|field| {
            (
                field,
                field
                    .attrs
                    .iter()
                    .find(|a| *a.path.get_ident().as_ref().unwrap() == SYSTEM_PARAM_ATTRIBUTE_NAME)
                    .map_or_else(SystemParamFieldAttributes::default, |a| {
                        syn::custom_keyword!(ignore);
                        let mut attributes = SystemParamFieldAttributes::default();
                        a.parse_args_with(|input: ParseStream| {
                            if input.parse::<Option<ignore>>()?.is_some() {
                                attributes.ignore = true;
                            }
                            Ok(())
                        })
                        .expect("Invalid 'render_resources' attribute format.");

                        attributes
                    }),
            )
        })
        .collect::<Vec<(&Field, SystemParamFieldAttributes)>>();
    let mut fields = Vec::new();
    let mut field_types = Vec::new();
    let mut ignored_fields = Vec::new();
    let mut ignored_field_types = Vec::new();
    for (field, attrs) in field_attributes.iter() {
        if attrs.ignore {
            ignored_fields.push(field.ident.as_ref().unwrap());
            ignored_field_types.push(&field.ty);
        } else {
            fields.push(field.ident.as_ref().unwrap());
            field_types.push(&field.ty);
        }
    }

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;
    let fetch_struct_name = Ident::new(&format!("Fetch{}", struct_name), Span::call_site());

    TokenStream::from(quote! {
        pub struct #fetch_struct_name;
        impl #impl_generics #path::SystemParam for #struct_name#ty_generics #where_clause {
            type Fetch = #fetch_struct_name;
        }

        impl #impl_generics #path::FetchSystemParam<'a> for #fetch_struct_name {
            type Item = #struct_name#ty_generics;
            fn init(system_state: &mut #path::SystemState, world: &#path::World, resources: &mut #path::Resources) {
                #(<<#field_types as SystemParam>::Fetch as #path::FetchSystemParam>::init(system_state, world, resources);)*
            }

            unsafe fn get_param(
                system_state: &'a #path::SystemState,
                world: &'a #path::World,
                resources: &'a #path::Resources,
            ) -> Option<Self::Item> {
                Some(#struct_name {
                    #(#fields: <<#field_types as SystemParam>::Fetch as #path::FetchSystemParam>::get_param(system_state, world, resources)?,)*
                    #(#ignored_fields: <#ignored_field_types>::default(),)*
                })
            }
        }
    })
}
