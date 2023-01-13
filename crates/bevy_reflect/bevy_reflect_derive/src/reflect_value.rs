use crate::container_attributes::ReflectTraits;
use crate::type_path::{parse_path_leading_colon, parse_path_no_leading_colon};
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Colon2, Paren, Where};
use syn::{parenthesized, Attribute, Generics, Path, PathSegment, Token};

/// A struct used to define a simple reflected value type (such as primitives).
///
///
///
/// This takes the form:
///
/// ```ignore
/// // Standard
/// ::my_crate::foo::Bar(TraitA, TraitB)
///
/// // With generics
/// ::my_crate::foo::Bar<T1: Bar, T2>(TraitA, TraitB)
///
/// // With generics and where clause
/// ::my_crate::foo::Bar<T1, T2> where T1: Bar (TraitA, TraitB)
/// ```
pub(crate) struct ReflectValueDef {
    #[allow(dead_code)]
    pub attrs: Vec<Attribute>,
    pub type_path: Path,
    pub generics: Generics,
    pub traits: Option<ReflectTraits>,
    pub alias: Option<Path>,
}

impl Parse for ReflectValueDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let type_path = Path::parse_mod_style(input)?;
        let mut generics = input.parse::<Generics>()?;
        generics.where_clause = input.parse()?;

        let mut traits = None;
        if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            traits = Some(content.parse::<ReflectTraits>()?);
        }
        
        let mut alias = None;
        if input.peek(Token![as]) {
            alias = Some(parse_path_no_leading_colon(input)?);
        };

        Ok(ReflectValueDef {
            attrs,
            type_path,
            generics: Generics {
                ..generics
            },
            traits,
            alias,
        })
    }
}
