use bevy_macro_utils::{get_lit_str, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident, Path, Result};

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

    let read_wrap = if attrs.change_detection_enabled {
        quote! { #bevy_ecs_path::change_detection::Ref<'a, Self> }
    } else {
        quote! { &'a Self }
    };

    let write_wrap = if attrs.change_detection_enabled {
        quote! { #bevy_ecs_path::change_detection::Mut<'a, Self> }
    } else {
        quote! { &'a mut Self }
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            type Storage = #storage;
            type ReadWrap<'a> = #read_wrap;
            fn shrink_read<'wlong: 'wshort, 'wshort>(item: Self::ReadWrap<'wlong>) -> Self::ReadWrap<'wshort> {
                item
            }
            type WriteWrap<'a> = #write_wrap;
            fn shrink_write<'wlong: 'wshort, 'wshort>(item: Self::WriteWrap<'wlong>) -> Self::WriteWrap<'wshort> {
                item
            }
        }
    })
}

pub const COMPONENT: Symbol = Symbol("component");
pub const STORAGE: Symbol = Symbol("storage");
pub const CHANGE_DETECTION: Symbol = Symbol("change_detection");

struct Attrs {
    change_detection_enabled: bool,
    storage: StorageTy,
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
    let meta_items = bevy_macro_utils::parse_attrs(ast, COMPONENT)?;

    let mut attrs = Attrs {
        change_detection_enabled: true,
        storage: StorageTy::Table,
    };

    for meta in meta_items {
        use syn::{
            Meta::NameValue,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(NameValue(m)) if m.path == STORAGE => {
                attrs.storage = match get_lit_str(STORAGE, &m.lit)?.value().as_str() {
                    TABLE => StorageTy::Table,
                    SPARSE_SET => StorageTy::SparseSet,
                    s => {
                        return Err(Error::new_spanned(
                            m.lit,
                            format!(
                                "Invalid storage type `{s}`, expected '{TABLE}' or '{SPARSE_SET}'.",
                            ),
                        ))
                    }
                };
            }
            Meta(NameValue(m)) if m.path == CHANGE_DETECTION => {
                attrs.change_detection_enabled = match m.lit {
                    syn::Lit::Bool(value) => value.value,
                    s => {
                        return Err(Error::new_spanned(
                            s,
                            "Change detection must be a bool, expected 'true' or 'false'.",
                        ))
                    }
                };
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown component attribute `{}`",
                        meta_item.path().into_token_stream()
                    ),
                ));
            }
            Lit(lit) => {
                return Err(Error::new_spanned(
                    lit,
                    "unexpected literal in component attribute",
                ))
            }
        }
    }

    Ok(attrs)
}

fn storage_path(bevy_ecs_path: &Path, ty: StorageTy) -> TokenStream2 {
    let typename = match ty {
        StorageTy::Table => Ident::new("TableStorage", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseStorage", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::#typename }
}
