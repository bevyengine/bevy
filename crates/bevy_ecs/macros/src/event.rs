use bevy_macro_utils::{get_lit_str, Symbol};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Path, Result};

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

    let missed = attrs.missed;

    let warn_missed = match missed {
        Missed::Ignore => quote! { const WARN_MISSED: bool = false; },
        Missed::DebugWarn => quote! {
            #[cfg(debug_assertions)]
            const WARN_MISSED: bool = true;
            #[cfg(not(debug_assertions))]
            const WARN_MISSED: bool = false;
        },
        Missed::Warn => quote! { const WARN_MISSED:bool = true; },
    };

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {
            #warn_missed
        }
    })
}

struct Attrs {
    missed: Missed,
}

enum Missed {
    Ignore,
    DebugWarn,
    Warn,
}

pub const EVENT: Symbol = Symbol("event");
pub const MISSED: Symbol = Symbol("missed");

const IGNORE: &str = "ignore";
const DEBUG_WARN: &str = "debug_warn";
const WARN: &str = "warn";

fn parse_event_attr(ast: &DeriveInput) -> Result<Attrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, EVENT)?;

    let mut attrs = Attrs {
        missed: Missed::DebugWarn,
    };

    for meta in meta_items {
        use syn::{
            Meta::NameValue,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(NameValue(m)) if m.path == MISSED => {
                attrs.missed = match get_lit_str(MISSED, &m.lit)?.value().as_str() {
                    IGNORE => Missed::Ignore,
                    DEBUG_WARN => Missed::DebugWarn,
                    WARN => Missed::Warn,
                    e => {
                        return Err(Error::new_spanned(
                            m.lit,
                            format!(
                                "Invalid missed event behaviour `{e}`, expected '{IGNORE}', '{DEBUG_WARN}', or '{WARN}'.",
                            ),
                        ))
                    }
                }
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown event attribute `{}`",
                        meta_item.path().into_token_stream()
                    ),
                ));
            }
            Lit(lit) => {
                return Err(Error::new_spanned(
                    lit,
                    "unexpected literal in event attribute",
                ))
            }
        }
    }

    Ok(attrs)
}
