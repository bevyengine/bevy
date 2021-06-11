use bevy_macro_utils::{get_lit_str, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident, Path, Result};

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_component_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let storage = storage_path(bevy_ecs_path.clone(), attrs.storage);

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

pub const COMPONENT: Symbol = Symbol("component");
pub const STORAGE: Symbol = Symbol("storage");

struct Attrs {
    storage: StorageTy,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

fn parse_component_attr(ast: &DeriveInput) -> Result<Attrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, COMPONENT)?;

    let mut attrs = Attrs {
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
                    "Table" => StorageTy::Table,
                    "SparseSet" => StorageTy::SparseSet,
                    s => {
                        return Err(Error::new_spanned(
                            m.lit,
                            format!(
                                "Invalid storage type `{}`, expected 'table' or 'sparse'.",
                                s
                            ),
                        ))
                    }
                };
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown component attribute `{}`",
                        meta_item.path().into_token_stream().to_string()
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

fn storage_path(bevy_ecs_path: Path, ty: StorageTy) -> TokenStream2 {
    let typename = match ty {
        StorageTy::Table => Ident::new("TableStorage", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseStorage", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::#typename }
}
