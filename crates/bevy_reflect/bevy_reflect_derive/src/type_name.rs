use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::Generics;

pub(crate) struct TypeNameDef {
    pub type_name: Ident,
    pub generics: Generics,
}

impl Parse for TypeNameDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let type_name = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;

        Ok(Self {
            type_name,
            generics,
        })
    }
}
