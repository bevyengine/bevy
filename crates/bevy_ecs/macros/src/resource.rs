use bevy_macro_utils::Symbol;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident, Path, Result};

pub fn derive_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_resource_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let is_setup_resource = Ident::new(&format!("{}", attrs.is_setup_resource), Span::call_site());

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::system::Resource for #struct_name #type_generics #where_clause {
            const IS_SETUP_RESOURCE: bool = #is_setup_resource;
        }
    })
}

pub const RESOURCE: Symbol = Symbol("resource");
pub const SETUP_RESOURCE: Symbol = Symbol("setup");

struct Attrs {
    is_setup_resource: bool,
}

fn parse_resource_attr(ast: &DeriveInput) -> Result<Attrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, RESOURCE)?;

    let mut attrs = Attrs {
        is_setup_resource: false,
    };

    for meta in meta_items {
        use syn::{
            Meta::Path,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(Path(m)) if m == SETUP_RESOURCE => attrs.is_setup_resource = true,
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
