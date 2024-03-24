use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, DeriveInput, Ident, LitBool, LitStr, Path, Result, TypeGenerics,
};

pub fn derive_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {
        }
    })
}

pub fn derive_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::system::Resource for #struct_name #type_generics #where_clause {
        }
    })
}

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_component_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let storage = storage_path(&bevy_ecs_path, attrs.storage);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let change_detection = attrs.change_detection;
    let write_item =
        write_item_associated_type(&bevy_ecs_path, struct_name, type_generics, change_detection);

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #storage;
            const CHANGE_DETECTION: bool = #change_detection;
            type WriteItem<'w> = #write_item;
            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::WriteItem<'wlong>) -> Self::WriteItem<'wshort> {
                item
            }
        }
    })
}

pub const COMPONENT: &str = "component";
pub const STORAGE: &str = "storage";
pub const CHANGE_DETECTION: &str = "change_detection";

struct Attrs {
    storage: StorageTy,
    change_detection: bool,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

// values for `storage` attribute
const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparseSet";

fn parse_component_attr(ast: &DeriveInput) -> Result<Attrs> {
    let mut attrs = Attrs {
        storage: StorageTy::Table,
        change_detection: true,
    };

    for meta in ast.attrs.iter().filter(|a| a.path().is_ident(COMPONENT)) {
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident(STORAGE) {
                attrs.storage = match nested.value()?.parse::<LitStr>()?.value() {
                    s if s == TABLE => StorageTy::Table,
                    s if s == SPARSE_SET => StorageTy::SparseSet,
                    s => {
                        return Err(nested.error(format!(
                            "Invalid storage type `{s}`, expected '{TABLE}' or '{SPARSE_SET}'.",
                        )));
                    }
                };
                Ok(())
            } else if nested.path.is_ident(CHANGE_DETECTION) {
                attrs.change_detection = nested.value()?.parse::<LitBool>()?.value();
                Ok(())
            } else {
                Err(nested.error("Unsupported attribute"))
            }
        })?;
    }

    Ok(attrs)
}

fn write_item_associated_type(
    bevy_ecs_path: &Path,
    struct_name: &Ident,
    type_generics: &TypeGenerics,
    change_detection: bool,
) -> TokenStream2 {
    if change_detection {
        quote! { #bevy_ecs_path::change_detection::Mut<'w, #struct_name #type_generics> }
    } else {
        quote! { &'w mut #struct_name #type_generics }
    }
}

fn storage_path(bevy_ecs_path: &Path, ty: StorageTy) -> TokenStream2 {
    let storage_type = match ty {
        StorageTy::Table => Ident::new("Table", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseSet", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::StorageType::#storage_type }
}
