use bevy_macro_utils::BevyManifest;
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Path};

pub(crate) fn bevy_asset_path() -> syn::Path {
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
    let mut field_visitors = Vec::new();
    if let Data::Struct(data_struct) = &ast.data {
        for field in &data_struct.fields {
            if field
                .attrs
                .iter()
                .any(|a| a.path().is_ident(DEPENDENCY_ATTRIBUTE))
            {
                if let Some(field_ident) = &field.ident {
                    field_visitors.push(quote! {
                        #bevy_asset_path::VisitAssetDependencies::visit_dependencies(&self.#field_ident, visit);
                    });
                }
            }
        }
    } else {
        return Err(syn::Error::new(
            Span::call_site().into(),
            "Asset derive currently only works on structs",
        ));
    }

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    // prevent unused variable warning in case there are no dependencies
    let visit = if field_visitors.is_empty() {
        quote! { _visit }
    } else {
        quote! { visit }
    };

    Ok(quote! {
        impl #impl_generics #bevy_asset_path::VisitAssetDependencies for #struct_name #type_generics #where_clause {
            fn visit_dependencies(&self, #visit: &mut impl FnMut(#bevy_asset_path::UntypedAssetId)) {
                #(#field_visitors)*
            }
        }
    })
}
