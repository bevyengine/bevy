use proc_macro2::Ident;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
    Generics, Path, PathSegment, Token,
};

pub(crate) fn parse_path_no_leading_colon(input: ParseStream) -> syn::Result<Path> {
    if input.peek(Token![::]) {
        return Err(input.error("did not expect a leading double colon (`::`)"));
    }

    let path = Path::parse_mod_style(input)?;

    if path.segments.is_empty() {
        Err(input.error("expected a path"))
    } else {
        Ok(path)
    }
}

/// An alias for a `TypePath`.
///
/// This is the parenthesized part of `(in my_crate::foo as MyType) SomeType`.
pub(crate) struct CustomPathDef {
    path: Path,
    name: Option<Ident>,
}

impl CustomPathDef {
    pub fn into_path(mut self, default_name: &Ident) -> Path {
        let name = PathSegment::from(self.name.unwrap_or_else(|| default_name.clone()));
        self.path.segments.push(name);
        self.path
    }

    pub fn parse_parenthesized(input: ParseStream) -> syn::Result<Option<Self>> {
        if input.peek(Paren) {
            let path;
            parenthesized!(path in input);
            Ok(Some(path.call(Self::parse)?))
        } else {
            Ok(None)
        }
    }

    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![in]>()?;

        let custom_path = parse_path_no_leading_colon(input)?;

        if !input.peek(Token![as]) {
            return Ok(Self {
                path: custom_path,
                name: None,
            });
        }

        input.parse::<Token![as]>()?;
        let custom_name: Ident = input.parse()?;

        Ok(Self {
            path: custom_path,
            name: Some(custom_name),
        })
    }
}

pub(crate) enum NamedTypePathDef {
    External {
        path: Path,
        generics: Generics,
        custom_path: Option<CustomPathDef>,
    },
    Primitive(Ident),
}

impl Parse for NamedTypePathDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let custom_path = CustomPathDef::parse_parenthesized(input)?;

        let path = Path::parse_mod_style(input)?;
        let mut generics = input.parse::<Generics>()?;
        generics.where_clause = input.parse()?;

        if path.leading_colon.is_none() && custom_path.is_none() {
            if path.segments.len() == 1 {
                let ident = path.segments.into_iter().next().unwrap().ident;
                Ok(NamedTypePathDef::Primitive(ident))
            } else {
                Err(input.error("non-customized paths must start with a double colon (`::`)"))
            }
        } else {
            Ok(NamedTypePathDef::External {
                path,
                generics,
                custom_path,
            })
        }
    }
}
