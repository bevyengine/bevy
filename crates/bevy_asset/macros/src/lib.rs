#![cfg_attr(docsrs, feature(doc_cfg))]

//! Macros for deriving asset traits.

use bevy_macro_utils::{as_member, BevyManifest};
use proc_macro::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Path};

pub(crate) fn bevy_asset_path() -> Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_asset"))
}

const DEPENDENCY_ATTRIBUTE: &str = "dependency";

/// Derive macro for the `Asset` trait.
///
/// Marks a type as a loadable asset. Also derives `VisitAssetDependencies`
/// automatically. Use `#[dependency]` on `Handle<T>` fields to declare asset
/// dependencies that should be loaded alongside this asset.
///
/// See the `Asset` trait docs for full explanation.
///
/// ```ignore
/// #[derive(Asset, TypePath)]
/// struct MyMaterial {
///     // This handle is tracked as a dependency.
///     #[dependency]
///     texture: Handle<Image>,
///     // Not a dependency, won't be tracked.
///     name: String,
/// }
/// ```
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

/// Derive macro for the `VisitAssetDependencies` trait.
///
/// Generates dependency-visiting code for asset types. This is automatically
/// derived when using `#[derive(Asset)]`, so you typically only need this
/// if implementing the `Asset` trait manually.
///
/// Use `#[dependency]` on `Handle<T>` fields to mark them as dependencies.
///
/// See the `VisitAssetDependencies` trait docs for full explanation.
///
/// ```ignore
/// #[derive(VisitAssetDependencies)]
/// struct MyData {
///     #[dependency]
///     texture: Handle<Image>,
/// }
/// ```
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
        Data::Struct(DataStruct { fields, .. }) => {
            let field_visitors = fields
                .iter()
                .enumerate()
                .filter(|(_, f)| field_has_dep(f))
                .map(|(i, field)| as_member(field.ident.as_ref(), i))
                .map(|member| visit_dep(quote!(&self.#member)));
            Some(quote!(#(#field_visitors)*))
        }
        Data::Enum(data_enum) => {
            let variant_has_dep = |v: &syn::Variant| v.fields.iter().any(field_has_dep);
            let any_case_required = data_enum.variants.iter().any(variant_has_dep);
            let cases = data_enum.variants.iter().filter(|v| variant_has_dep(v));
            let cases = cases.map(|variant| {
                let ident = &variant.ident;
                let field_members = variant
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| field_has_dep(f))
                    .map(|(i, field)| as_member(field.ident.as_ref(), i));
                let field_locals = field_members.clone().map(|m| format_ident!("__self_{}", m));
                let field_visitors = field_locals.clone().map(|i| visit_dep(quote!(#i)));
                quote!(Self::#ident {#(#field_members: #field_locals,)* ..} => {
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
