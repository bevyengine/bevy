use syn::{parenthesized, parse::Parse, token::Paren, ExprClosure, Path, Result};

pub(crate) mod kw {
    syn::custom_keyword!(from);
}

#[derive(Clone)]
pub(crate) struct Conversion {
    pub(crate) path: Path,
    pub(crate) func: Option<ExprClosure>,
}

impl Conversion {
    pub(crate) fn parse_from_attr(input: syn::parse::ParseStream) -> Result<Self> {
        input.parse::<kw::from>()?;
        let conversion;
        parenthesized!(conversion in input);
        conversion.parse()
    }
}

impl Parse for Conversion {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let path = input.parse::<Path>()?;
        let func = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            Some(content.parse::<ExprClosure>()?)
        } else {
            None
        };
        Ok(Self { path, func })
    }
}
