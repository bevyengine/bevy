// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use bevy_macro_utils::BevyManifest;
use proc_macro::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Path};

pub(crate) fn bevy_asset_path() -> Path {
    BevyManifest::default().get_path("bevy_asset")
}

const DEPENDENCY_ATTRIBUTE: &str = "dependency";

#[proc_macro_derive(Asset, attributes(dependency))]
pub fn derive_asset(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_asset_path: Path = bevy_asset_path();

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let dependency_visitor = match derive_dependency_visitor_internal(&ast, &bevy_asset_path) {
        Ok(dependency_visitor) => dependency_visitor,
        Err(err) => return err.into_compile_error().into(),
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_asset_path::Asset for #struct_name #type_generics #where_clause { }
        #dependency_visitor
    })
}

#[proc_macro_derive(VisitAssetDependencies, attributes(dependency))]
pub fn derive_asset_dependency_visitor(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_asset_path: Path = bevy_asset_path();
    match derive_dependency_visitor_internal(&ast, &bevy_asset_path) {
        Ok(dependency_visitor) => TokenStream::from(dependency_visitor),
        Err(err) => err.into_compile_error().into(),
    }
}

fn derive_dependency_visitor_internal(
    ast: &DeriveInput,
    bevy_asset_path: &Path,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let visit_dep = |to_read| quote!(#bevy_asset_path::VisitAssetDependencies::visit_dependencies(#to_read, visit););
    let is_dep_attribute = |a: &syn::Attribute| a.path().is_ident(DEPENDENCY_ATTRIBUTE);
    let field_has_dep = |f: &syn::Field| f.attrs.iter().any(is_dep_attribute);

    let body = match &ast.data {
        Data::Struct(data_struct) => {
            let fields = data_struct.fields.iter();
            let field_visitors = fields.enumerate().filter(|(_, f)| field_has_dep(f));
            let field_visitors = field_visitors.map(|(i, field)| match &field.ident {
                Some(ident) => visit_dep(quote!(&self.#ident)),
                None => {
                    let index = syn::Index::from(i);
                    visit_dep(quote!(&self.#index))
                }
            });
            Some(quote!( #(#field_visitors)* ))
        }
        Data::Enum(data_enum) => {
            let variant_has_dep = |v: &syn::Variant| v.fields.iter().any(field_has_dep);
            let any_case_required = data_enum.variants.iter().any(variant_has_dep);
            let cases = data_enum.variants.iter().filter(|v| variant_has_dep(v));
            let cases = cases.map(|variant| {
                let ident = &variant.ident;
                let fields = &variant.fields;

                let field_visitors = fields.iter().enumerate().filter(|(_, f)| field_has_dep(f));

                let field_visitors = field_visitors.map(|(i, field)| match &field.ident {
                    Some(ident) => visit_dep(quote!(#ident)),
                    None => {
                        let ident = format_ident!("member{i}");
                        visit_dep(quote!(#ident))
                    }
                });
                let fields = match fields {
                    syn::Fields::Named(fields) => {
                        let named = fields.named.iter().map(|f| f.ident.as_ref());
                        quote!({ #(#named,)* .. })
                    }
                    syn::Fields::Unnamed(fields) => {
                        let named = (0..fields.unnamed.len()).map(|i| format_ident!("member{i}"));
                        quote!( ( #(#named,)* ) )
                    }
                    syn::Fields::Unit => unreachable!("Can't pass filter is_dep_attribute"),
                };
                quote!(Self::#ident #fields => {
                    #(#field_visitors)*
                })
            });

            any_case_required.then(|| quote!(match self { #(#cases)*, _ => {} }))
        }
        Data::Union(_) => {
            return Err(syn::Error::new(
                Span::call_site().into(),
                "Asset derive currently doesn't work on unions",
            ));
        }
    };

    // prevent unused variable warning in case there are no dependencies
    let visit = if body.is_none() {
        quote! { _visit }
    } else {
        quote! { visit }
    };

    Ok(quote! {
        impl #impl_generics #bevy_asset_path::VisitAssetDependencies for #struct_name #type_generics #where_clause {
            fn visit_dependencies(&self, #visit: &mut impl FnMut(#bevy_asset_path::UntypedAssetId)) {
                #body
            }
        }
    })
}
