use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Ident, LitStr, Path, Result};

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
    let compose_function = attrs.compose.into_iter();
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            type Storage = #storage;

            #(fn compose(&mut self, incoming: Self) {
                #compose_function(self, incoming)
            })*
        }
    })
}

pub const COMPONENT: &str = "component";
pub const STORAGE: &str = "storage";
pub const COMPOSE: &str = "compose";

struct Attrs {
    storage: StorageTy,
    compose: Option<Path>,
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
        compose: None,
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
            } else if nested.path.is_ident(COMPOSE) {
                let path = nested.value()?.parse::<LitStr>()?.value();
                match syn::parse_str(&path) {
                    Ok(s) => attrs.compose = Some(s),
                    Err(_) => {
                        return Err(nested.error(format!(
                            "Invalid compose function `{path}`, expected a function.",
                        )))
                    }
                }
                Ok(())
            } else {
                Err(nested.error("Unsupported attribute"))
            }
        })?;
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
