use crate::container_attributes::ReflectTraits;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Colon2, Paren, Where};
use syn::{parenthesized, Attribute, Generics, Path, PathSegment};

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
}

impl Parse for ReflectValueDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let type_path = {
            let lookahead = input.lookahead1();
            if lookahead.peek(Colon2) {
                // This parses `::foo::Foo` from `::foo::Foo<T>` (leaving the generics).
                Path::parse_mod_style(input)?
            } else {
                let ident = input.parse::<Ident>()?;
                let mut segments = Punctuated::new();
                segments.push(PathSegment::from(ident));
                Path {
                    leading_colon: None,
                    segments,
                }
            }
        };
        let generics = input.parse::<Generics>()?;
        let mut lookahead = input.lookahead1();
        let mut where_clause = None;
        if lookahead.peek(Where) {
            where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        let mut traits = None;
        if lookahead.peek(Paren) {
            let content;
            parenthesized!(content in input);
            traits = Some(content.parse::<ReflectTraits>()?);
        }

        Ok(ReflectValueDef {
            attrs,
            type_path,
            generics: Generics {
                where_clause,
                ..generics
            },
            traits,
        })
    }
}
