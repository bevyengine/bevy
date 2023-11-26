use crate::container_attributes::ReflectTraits;
use crate::type_path::CustomPathDef;
use syn::parse::{Parse, ParseStream};
use syn::token::Paren;
use syn::{parenthesized, Attribute, Generics, Path};

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
///
/// // With a custom path (not with impl_from_reflect_value)
/// (in my_crate::bar) Bar(TraitA, TraitB)
/// ```
pub(crate) struct ReflectValueDef {
    #[allow(dead_code)]
    pub attrs: Vec<Attribute>,
    pub type_path: Path,
    pub generics: Generics,
    pub traits: Option<ReflectTraits>,
    pub custom_path: Option<CustomPathDef>,
}

impl Parse for ReflectValueDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;

        let custom_path = CustomPathDef::parse_parenthesized(input)?;

        let type_path = Path::parse_mod_style(input)?;
        let mut generics = input.parse::<Generics>()?;
        generics.where_clause = input.parse()?;

        let mut traits = None;
        if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            traits = Some(content.parse::<ReflectTraits>()?);
        }
        Ok(ReflectValueDef {
            attrs,
            type_path,
            generics,
            traits,
            custom_path,
        })
    }
}
