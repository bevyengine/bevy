use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, LitBool, Pat, Path, Result};

use crate::bevy_state_path;

pub const STATES: &str = "states";
pub const SCOPED_ENTITIES: &str = "scoped_entities";

struct StatesAttrs {
    scoped_entities_enabled: bool,
}

fn parse_states_attr(ast: &DeriveInput) -> Result<StatesAttrs> {
    let mut attrs = StatesAttrs {
        scoped_entities_enabled: true,
    };

    for attr in ast.attrs.iter() {
        if attr.path().is_ident(STATES) {
            attr.parse_nested_meta(|nested| {
                if nested.path.is_ident(SCOPED_ENTITIES) {
                    if let Ok(value) = nested.value() {
                        attrs.scoped_entities_enabled = value.parse::<LitBool>()?.value();
                    }
                    Ok(())
                } else {
                    Err(nested.error("Unsupported attribute"))
                }
            })?;
        }
    }

    Ok(attrs)
}

pub fn derive_states(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let attrs = match parse_states_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut base_trait_path = bevy_state_path();
    base_trait_path.segments.push(format_ident!("state").into());

    let mut trait_path = base_trait_path.clone();
    trait_path.segments.push(format_ident!("States").into());

    let mut state_mutation_trait_path = base_trait_path.clone();
    state_mutation_trait_path
        .segments
        .push(format_ident!("FreelyMutableState").into());

    let struct_name = &ast.ident;

    let scoped_entities_enabled = attrs.scoped_entities_enabled;

    quote! {
        impl #impl_generics #trait_path for #struct_name #ty_generics #where_clause {
            const SCOPED_ENTITIES_ENABLED: bool = #scoped_entities_enabled;
        }

        impl #impl_generics #state_mutation_trait_path for #struct_name #ty_generics #where_clause {
        }
    }
    .into()
}

struct Source {
    source_type: Path,
    source_value: Pat,
}

fn parse_sources_attr(ast: &DeriveInput) -> Result<(StatesAttrs, Source)> {
    let mut result = ast
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("source"))
        .map(|meta| {
            let mut source = None;
            let value = meta.parse_nested_meta(|nested| {
                let source_type = nested.path.clone();
                let source_value = Pat::parse_multi(nested.value()?)?;
                source = Some(Source {
                    source_type,
                    source_value,
                });
                Ok(())
            });
            match source {
                Some(value) => Ok(value),
                None => match value {
                    Ok(_) => Err(syn::Error::new(
                        ast.span(),
                        "Couldn't parse SubStates source",
                    )),
                    Err(e) => Err(e),
                },
            }
        })
        .collect::<Result<Vec<_>>>()?;

    if result.len() > 1 {
        return Err(syn::Error::new(
            ast.span(),
            "Only one source is allowed for SubStates",
        ));
    }

    let states_attrs = parse_states_attr(ast)?;

    let Some(result) = result.pop() else {
        return Err(syn::Error::new(ast.span(), "SubStates require a source"));
    };

    Ok((states_attrs, result))
}

pub fn derive_substates(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let (states_attrs, sources) =
        parse_sources_attr(&ast).expect("Failed to parse substate sources");

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut base_trait_path = bevy_state_path();
    base_trait_path.segments.push(format_ident!("state").into());

    let mut trait_path = base_trait_path.clone();
    trait_path.segments.push(format_ident!("SubStates").into());

    let mut state_set_trait_path = base_trait_path.clone();
    state_set_trait_path
        .segments
        .push(format_ident!("StateSet").into());

    let mut state_trait_path = base_trait_path.clone();
    state_trait_path
        .segments
        .push(format_ident!("States").into());

    let mut state_mutation_trait_path = base_trait_path.clone();
    state_mutation_trait_path
        .segments
        .push(format_ident!("FreelyMutableState").into());

    let struct_name = &ast.ident;

    let source_state_type = sources.source_type;
    let source_state_value = sources.source_value;

    let scoped_entities_enabled = states_attrs.scoped_entities_enabled;

    let result = quote! {
        impl #impl_generics #trait_path for #struct_name #ty_generics #where_clause {
            type SourceStates = #source_state_type;

            fn should_exist(sources: #source_state_type) -> Option<Self> {
                matches!(sources, #source_state_value).then_some(Self::default())
            }
        }

        impl #impl_generics #state_trait_path for #struct_name #ty_generics #where_clause {
            const DEPENDENCY_DEPTH : usize = <Self as #trait_path>::SourceStates::SET_DEPENDENCY_DEPTH + 1;

            const SCOPED_ENTITIES_ENABLED: bool = #scoped_entities_enabled;
        }

        impl #impl_generics #state_mutation_trait_path for #struct_name #ty_generics #where_clause {
        }
    };

    // panic!("Got Result\n{}", result.to_string());

    result.into()
}
