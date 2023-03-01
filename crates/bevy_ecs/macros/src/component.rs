use bevy_macro_utils::{get_lit_str, NamedArg, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
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
                #(_sets.push(Box::new(#write_sets));)*
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
    let mut attrs = Attrs {
        storage: StorageTy::Table,
        read_sets: vec![],
        write_sets: vec![],
    };

    for NamedArg { path, expr } in bevy_macro_utils::parse_attrs(ast, COMPONENT)? {
        match path.get_ident().unwrap().to_string().as_str() {
            "storage" => {
                attrs.storage = match get_lit_str(STORAGE, &expr)?.value().as_str() {
                    TABLE => StorageTy::Table,
                    SPARSE_SET => StorageTy::SparseSet,
                    s => {
                        return Err(Error::new_spanned(
                            expr,
                            format!(
                                "Invalid storage type `{s}`, expected '{TABLE}' or '{SPARSE_SET}'.",
                            ),
                        ))
                    }
                };
            }
            "read_set" => attrs.read_sets.push(expr),
            "write_set" => attrs.write_sets.push(expr),
            _ => {
                return Err(Error::new_spanned(
                    path,
                    "Invalid component attribute format: expected `storages`, `read_set`, or `write_set`",
                ));
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
