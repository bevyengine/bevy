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

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::crate_name;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Path};

/// Implement `Bundle` for a monomorphic struct
///
/// Using derived `Bundle` impls improves spawn performance and can be convenient when combined with
/// other derives like `serde::Deserialize`.
#[allow(clippy::cognitive_complexity)]
#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if !input.generics.params.is_empty() {
        return TokenStream::from(
            quote! { compile_error!("derive(Bundle) does not support generics"); },
        );
    }
    let data = match input.data {
        syn::Data::Struct(s) => s,
        _ => {
            return TokenStream::from(
                quote! { compile_error!("derive(Bundle) only supports structs"); },
            )
        }
    };
    let ident = input.ident;
    let (tys, fields) = struct_fields(&data.fields);
    let path_str = if crate_name("bevy").is_ok() {
        "bevy::ecs"
    } else if crate_name("bevy_ecs").is_ok() {
        "bevy_ecs"
    } else {
        "bevy_hecs"
    };

    let path: Path = syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap();

    let n = tys.len();
    let code = quote! {
        impl #path::DynamicBundle for #ident {
            fn with_ids<T>(&self, f: impl FnOnce(&[std::any::TypeId]) -> T) -> T {
                Self::with_static_ids(f)
            }

            fn type_info(&self) -> Vec<#path::TypeInfo> {
                Self::static_type_info()
            }

            unsafe fn put(mut self, mut f: impl FnMut(*mut u8, std::any::TypeId, usize) -> bool) {
                #(
                    if f((&mut self.#fields as *mut #tys).cast::<u8>(), std::any::TypeId::of::<#tys>(), std::mem::size_of::<#tys>()) {
                        #[allow(clippy::forget_copy)]
                        std::mem::forget(self.#fields);
                    }
                )*
            }
        }

        impl #path::Bundle for #ident {
            fn with_static_ids<T>(f: impl FnOnce(&[std::any::TypeId]) -> T) -> T {
                use std::any::TypeId;
                use std::mem;

                #path::lazy_static::lazy_static! {
                    static ref ELEMENTS: [TypeId; #n] = {
                        let mut dedup = #path::bevy_utils::HashSet::default();
                        for &(ty, name) in [#((std::any::TypeId::of::<#tys>(), std::any::type_name::<#tys>())),*].iter() {
                            if !dedup.insert(ty) {
                                panic!("{} has multiple {} fields; each type must occur at most once!", stringify!(#ident), name);
                            }
                        }

                        let mut tys = [#((mem::align_of::<#tys>(), TypeId::of::<#tys>())),*];
                        tys.sort_unstable_by(|x, y| x.0.cmp(&y.0).reverse().then(x.1.cmp(&y.1)));
                        let mut ids = [TypeId::of::<()>(); #n];
                        for (id, info) in ids.iter_mut().zip(tys.iter()) {
                            *id = info.1;
                        }
                        ids
                    };
                }

                f(&*ELEMENTS)
            }

            fn static_type_info() -> Vec<#path::TypeInfo> {
                let mut info = vec![#(#path::TypeInfo::of::<#tys>()),*];
                info.sort_unstable();
                info
            }

            unsafe fn get(
                mut f: impl FnMut(std::any::TypeId, usize) -> Option<std::ptr::NonNull<u8>>,
            ) -> Result<Self, #path::MissingComponent> {
                #(
                    let #fields = f(std::any::TypeId::of::<#tys>(), std::mem::size_of::<#tys>())
                            .ok_or_else(#path::MissingComponent::new::<#tys>)?
                            .cast::<#tys>()
                        .as_ptr();
                )*
                Ok(Self { #( #fields: #fields.read(), )* })
            }
        }
    };
    TokenStream::from(code)
}

fn struct_fields(fields: &syn::Fields) -> (Vec<&syn::Type>, Vec<syn::Ident>) {
    match fields {
        syn::Fields::Named(ref fields) => fields
            .named
            .iter()
            .map(|f| (&f.ty, f.ident.clone().unwrap()))
            .unzip(),
        syn::Fields::Unnamed(ref fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| (&f.ty, syn::Ident::new(&i.to_string(), Span::call_site())))
            .unzip(),
        syn::Fields::Unit => (Vec::new(), Vec::new()),
    }
}
