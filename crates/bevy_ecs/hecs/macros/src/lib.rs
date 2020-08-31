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
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Path};

/// Implement `Bundle` for a monomorphic struct
///
/// Using derived `Bundle` impls improves spawn performance and can be convenient when combined with
/// other derives like `serde::Deserialize`.
///
/// Attributes: `#[bundle(skip)]` on fields, skips that field. Requires that the field type
/// implements `Default`
#[allow(clippy::cognitive_complexity)]
#[proc_macro_derive(Bundle, attributes(bundle))]
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
    let (fields, skipped) = match struct_fields(&data.fields) {
        Ok(fields) => fields,
        Err(stream) => return stream,
    };
    let tys = fields.iter().map(|f| f.0).collect::<Vec<_>>();
    let field_names = fields.iter().map(|f| f.1.clone()).collect::<Vec<_>>();
    let path_str = if crate_name("bevy").is_ok() {
        "bevy::ecs"
    } else if crate_name("bevy_ecs").is_ok() {
        "bevy_ecs"
    } else {
        "bevy_hecs"
    };

    let path: Path = syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap();

    let n = fields.len();
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
                    if f((&mut self.#field_names as *mut #tys).cast::<u8>(), std::any::TypeId::of::<#tys>(), std::mem::size_of::<#tys>()) {
                        #[allow(clippy::forget_copy)]
                        std::mem::forget(self.#field_names);
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
                    let #field_names = f(std::any::TypeId::of::<#tys>(), std::mem::size_of::<#tys>())
                            .ok_or_else(#path::MissingComponent::new::<#tys>)?
                            .cast::<#tys>()
                        .as_ptr();
                )*
                Ok(Self { #( #field_names: #field_names.read(), )* #(#skipped: Default::default(),)* })
            }
        }
    };
    dbg!(code.to_string());
    TokenStream::from(code)
}

fn struct_fields(
    fields: &syn::Fields,
) -> Result<(Vec<(&syn::Type, syn::Ident)>, Vec<syn::Ident>), TokenStream> {
    let mut final_fields = Vec::new();
    let mut skipped = Vec::new();
    match fields {
        syn::Fields::Named(ref fields) => {
            for field in &fields.named {
                if should_include_in_bundle(field)? {
                    final_fields.push((&field.ty, field.ident.clone().unwrap()));
                } else {
                    skipped.push(field.ident.clone().unwrap());
                }
            }
        }
        syn::Fields::Unnamed(ref fields) => {
            for (i, field) in fields.unnamed.iter().enumerate() {
                if should_include_in_bundle(field)? {
                    final_fields.push((
                        &field.ty,
                        syn::Ident::new(&i.to_string(), Span::call_site()),
                    ));
                } else {
                    skipped.push(syn::Ident::new(&i.to_string(), Span::call_site()));
                }
            }
        }
        syn::Fields::Unit => {}
    };
    return Ok((final_fields, skipped));
}

fn should_include_in_bundle(f: &syn::Field) -> Result<bool, TokenStream> {
    for attr in &f.attrs {
        if attr.path.is_ident("bundle") {
            let string = attr.tokens.to_string();
            if attr.tokens.to_string() == "(skip)" {
                return Ok(false);
            } else {
                let error = format!("Invalid bundle attribute #[bundle{}]", string);
                return Err(quote_spanned! {attr.span().into() => compile_error!(#error)}.into());
            }
        }
    }
    return Ok(true);
}
