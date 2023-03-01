use bevy_macro_utils::{get_lit_str, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
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

    let read_sets = attrs.read_sets;
    let write_sets = attrs.write_sets;

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            type Storage = #storage;

            fn add_read_sets(_sets: &mut Vec<#bevy_ecs_path::schedule::BoxedSystemSet>) {
                #(_sets.push(#read_sets.dyn_clone());)*
            }

            fn add_write_sets(_sets: &mut Vec<#bevy_ecs_path::schedule::BoxedSystemSet>) {
                #(_sets.push(#write_sets.dyn_clone());)*
            }
        }
    })
}

pub const COMPONENT: Symbol = Symbol("component");
pub const STORAGE: Symbol = Symbol("storage");

struct Attrs {
    storage: StorageTy,
    read_sets: Vec<syn::Expr>,
    write_sets: Vec<syn::Expr>,
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
        storage: StorageTy::Table,
        read_sets: vec![],
        write_sets: vec![],
    };

    for meta in meta_items {
        let syn::ExprAssign { left, right, .. } = syn::parse2(meta)?;
        let left = match *left {
            syn::Expr::Path(left) => left,
            other => {
                return Err(Error::new_spanned(
                    other,
                    r#"invalid attribute: expected #[component(storage = "...")]"#,
                ))
            }
        };
        let left_ident = left.path.get_ident().unwrap();
        if left_ident == &format_ident!("storage") {
            attrs.storage = match get_lit_str(STORAGE, &right)?.value().as_str() {
                TABLE => StorageTy::Table,
                SPARSE_SET => StorageTy::SparseSet,
                s => {
                    return Err(Error::new_spanned(
                        right,
                        format!(
                            "Invalid storage type `{s}`, expected '{TABLE}' or '{SPARSE_SET}'.",
                        ),
                    ))
                }
            };
        } else if left_ident == &format_ident!("read_set") {
            attrs.read_sets.push(*right);
        } else if left_ident == &format_ident!("write_set") {
            attrs.write_sets.push(*right);
        } else {
            return Err(Error::new_spanned(
                left,
                "Invalid component attribute format: expected `storages`",
            ));
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
