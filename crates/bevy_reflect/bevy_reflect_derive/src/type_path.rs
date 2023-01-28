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

pub struct CustomPathDef {
    custom_path: Option<Path>,
    custom_name: Option<Ident>,
}

impl CustomPathDef {
    pub fn is_empty(&self) -> bool {
        self.custom_path.is_none()
    }

    pub fn into_path(self, default_name: &Ident) -> Option<Path> {
        if let Some(mut path) = self.custom_path {
            let name = PathSegment::from(self.custom_name.unwrap_or_else(|| default_name.clone()));
            path.segments.push(name);
            Some(path)
        } else {
            None
        }
    }
}

impl Parse for CustomPathDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(Token![in]) {
            return Ok(Self {
                custom_path: None,
                custom_name: None,
            });
        }

        input.parse::<Token![in]>()?;
        let custom_path = parse_path_no_leading_colon(input)?;

        if !input.peek(Token![as]) {
            return Ok(Self {
                custom_path: Some(custom_path),
                custom_name: None,
            });
        }

        input.parse::<Token![as]>()?;
        let custom_name: Ident = input.parse()?;

        Ok(Self {
            custom_path: Some(custom_path),
            custom_name: Some(custom_name),
        })
    }
}

pub enum NamedTypePathDef {
    External {
        path: Path,
        generics: Generics,
        custom_path: CustomPathDef,
    },
    Primtive(Ident),
}

impl Parse for NamedTypePathDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = Path::parse_mod_style(input)?;
        let mut generics = input.parse::<Generics>()?;
        generics.where_clause = input.parse()?;

        let custom_path: CustomPathDef = input.parse()?;

        if path.leading_colon.is_none() && custom_path.is_empty() {
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
                custom_path,
            })
        }
    }
}
