use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    Generics, Path, PathSegment, Token,
};

pub fn parse_path_no_leading_colon(input: ParseStream) -> syn::Result<Path> {
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

pub struct AliasDef {
    path_alias: Option<Path>,
    name_alias: Option<Ident>,
}

impl AliasDef {
    pub fn is_empty(&self) -> bool {
        self.path_alias.is_none()
    }

    pub fn into_path(self, default_name: &Ident) -> Option<Path> {
        if let Some(mut path) = self.path_alias {
            let name = PathSegment::from(self.name_alias.unwrap_or_else(|| default_name.clone()));
            path.segments.push(name);
            Some(path)
        } else {
            None
        }
    }
}

impl Parse for AliasDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(Token![in]) {
            return Ok(Self {
                path_alias: None,
                name_alias: None,
            });
        }

        input.parse::<Token![in]>()?;
        let path_alias = parse_path_no_leading_colon(input)?;

        if !input.peek(Token![as]) {
            return Ok(Self {
                path_alias: Some(path_alias),
                name_alias: None,
            });
        }

        input.parse::<Token![as]>()?;
        let name_alias: Ident = input.parse()?;

        Ok(Self {
            path_alias: Some(path_alias),
            name_alias: Some(name_alias),
        })
    }
}

pub enum NamedTypePathDef {
    External {
        path: Path,
        generics: Generics,
        alias: AliasDef,
    },
    Primtive(Ident),
}

impl Parse for NamedTypePathDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = Path::parse_mod_style(input)?;
        let mut generics = input.parse::<Generics>()?;
        generics.where_clause = input.parse()?;

        let alias: AliasDef = input.parse()?;

        if path.leading_colon.is_none() && alias.is_empty() {
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
                alias,
            })
        }
    }
}
