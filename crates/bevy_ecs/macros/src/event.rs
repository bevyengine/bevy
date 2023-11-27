use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, LitStr, Path, Result};

pub fn derive_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let attrs = match parse_event_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let storage_ty = match attrs.storage {
        StorageTy::Vec => quote! {
            type Storage = ::std::vec::Vec<#bevy_ecs_path::event::EventInstance<Self>>;
        },
        StorageTy::SmallVec(size) => quote! {
            type Storage = #bevy_ecs_path::__macro_export::SmallVec<[#bevy_ecs_path::event::EventInstance<Self>; #size]>;
        },
    };

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {
            #storage_ty
        }
    })
}

struct Attrs {
    storage: StorageTy,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Vec,
    SmallVec(usize),
}

pub const EVENT: &str = "event";
pub const STORAGE: &str = "storage";

fn parse_event_attr(ast: &DeriveInput) -> Result<Attrs> {
    // let meta_items = bevy_macro_utils::parse_attrs(ast, EVENT)?;
    let mut attrs = Attrs {
        storage: StorageTy::Vec,
    };

    for meta in ast.attrs.iter().filter(|a| a.path().is_ident(EVENT)) {
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident(STORAGE) {
                attrs.storage = match nested.value()?.parse::<LitStr>()?.value() {
                    s if s == "Vec" => StorageTy::Vec,
                    s if s.starts_with("SmallVec(") && s.ends_with(')') => {
                        let trimmed = &s["SmallVec(".len()..][..1];
                        match trimmed.parse::<usize>() {
                            Ok(size) => StorageTy::SmallVec(size),
                            Err(_) => {
                                return Err(Error::new_spanned(
                                    nested.path,
                                    format!("Invalid smallvec size {trimmed}."),
                                ))
                            }
                        }
                    }
                    s => {
                        return Err(nested.error(format!(
                            "Invalid storage type `{s}`, expected 'Vec' or 'SmallVec(n)'.",
                        )));
                    }
                };
                Ok(())
            } else {
                Err(nested.error("Unsupported attribute"))
            }
        })?;
    }

    // for meta in meta_items {
    //     use syn::{
    //         Meta::NameValue,
    //         NestedMeta::{Lit, Meta},
    //     };
    //     match meta {
    //         Meta(NameValue(m)) if m.path == STORAGE => {
    //             attrs.storage = match get_lit_str(STORAGE, &m.lit)?.value().as_str() {
    //                 "vec" => StorageTy::Vec,
    //                 lit if lit.starts_with("smallvec(") && lit.ends_with(')') => {
    //                     let trimmed = &lit["smallvec(".len()..][..1];
    //                     match trimmed.parse::<usize>() {
    //                         Ok(size) => StorageTy::SmallVec(size),
    //                         Err(_) => {
    //                             return Err(Error::new_spanned(
    //                                 m.lit,
    //                                 format!("Invalid smallvec size {trimmed}."),
    //                             ))
    //                         }
    //                     }
    //                 }
    //                 e => {
    //                     return Err(Error::new_spanned(
    //                         m.lit,
    //                         format!(
    //                     "Invalid storage type behaviour `{e}`, expected 'vec' or 'smallvec(N)'.",
    //                 ),
    //                     ))
    //                 }
    //             }
    //         }
    //         Meta(meta_item) => {
    //             return Err(Error::new_spanned(
    //                 meta_item.path(),
    //                 format!(
    //                     "unknown event attribute `{}`",
    //                     meta_item.path().into_token_stream()
    //                 ),
    //             ));
    //         }
    //         Lit(lit) => {
    //             return Err(Error::new_spanned(
    //                 lit,
    //                 "unexpected literal in event attribute",
    //             ))
    //         }
    //     }
    // }

    Ok(attrs)
}
