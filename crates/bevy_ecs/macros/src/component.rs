use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, parse_quote, Attribute, DeriveInput, Error, Ident, Path, Result};

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let storage = ast
        .attrs
        .iter()
        .find(|attr| attr.path.is_ident("storage"))
        .map_or(Ok(StorageTy::Table), parse_storage_attribute)
        .map(|ty| storage_path(bevy_ecs_path.clone(), ty))
        .unwrap_or_else(|err| err.to_compile_error());

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            type Storage = #storage;
        }
    })
}

enum StorageTy {
    Table,
    Sparse,
}

fn parse_storage_attribute(attr: &Attribute) -> Result<StorageTy> {
    let ident = attr.parse_args::<Ident>()?;
    match ident.to_string().as_str() {
        "table" => Ok(StorageTy::Table),
        "sparse" => Ok(StorageTy::Sparse),
        _ => Err(Error::new(
            ident.span(),
            "Invalid storage type, expected 'table' or 'sparse'.",
        )),
    }
}

fn storage_path(bevy_ecs_path: Path, ty: StorageTy) -> TokenStream2 {
    let typename = match ty {
        StorageTy::Table => Ident::new("TableStorage", Span::call_site()),
        StorageTy::Sparse => Ident::new("SparseStorage", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::#typename }
}
