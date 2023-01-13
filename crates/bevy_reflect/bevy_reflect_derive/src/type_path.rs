use std::path;

use proc_macro2::Ident;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Expr, Generics, LitStr, Path, Token, Type, TypeArray, TypeParamBound, TypePath,
};

use crate::derive_data::ReflectMeta;

pub enum NamedTypePathDef {
    External {
        path: Path,
        generics: Generics,
        alias: Option<Path>,
    },
    Primtive(Ident),
}

pub struct AnonymousTypePathDef {
    pub path: Path,
    pub generics: Generics,
    pub long_type_path: Expr,
    pub short_type_path: Expr,
}

pub fn parse_path_leading_colon(input: ParseStream) -> syn::Result<Path> {
    let leading = input.parse::<Token![::]>()?;

    if input.peek(Token![::]) {
        return Err(input.error("did not expect two leading double colons (`::::`)"));
    }

    let mut path = Path::parse_mod_style(input)?;
    path.leading_colon = Some(leading);
    Ok(path)
}

pub fn parse_path_no_leading_colon(input: ParseStream) -> syn::Result<Path> {
    if input.peek(Token![::]) {
        return Err(input.error("did not expect a leading double colon (`::`)"));
    }

    Path::parse_mod_style(input)
}

impl Parse for NamedTypePathDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = Path::parse_mod_style(input)?;
        let mut generics = input.parse::<Generics>()?;
        generics.where_clause = input.parse()?;

        if input.is_empty() {
            if path.leading_colon.is_none() {
                if path.segments.len() == 1 {
                    let ident = path.segments.into_iter().next().unwrap().ident;
                    Ok(NamedTypePathDef::Primtive(ident))
                } else {
                    Err(input.error("non-aliased paths must start with a double colon (`::`)"))
                }
            } else {
            Ok(NamedTypePathDef::External {
                path,
                generics,
                alias: None,
            })
            }
            
        } else {
            let _ = input.parse::<Token![as]>();
            let alias = parse_path_no_leading_colon(input)?;
            Ok(NamedTypePathDef::External {
                path,
                generics,
                alias: Some(alias),
            })
        }
    }
}
