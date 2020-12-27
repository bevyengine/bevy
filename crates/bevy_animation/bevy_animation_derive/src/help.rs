use proc_macro2::Span;
use syn::{
    parse::{Error as ParseError, Parse, ParseStream, Result as ParserResult},
    punctuated::Punctuated,
    token::Comma,
    Attribute, Field,
};

pub fn snake_to_pascal_case(name: &str) -> String {
    let mut n = String::new();

    name.split('_').for_each(|s| {
        s.chars().enumerate().for_each(|(i, c)| {
            if i == 0 {
                n.extend(c.to_uppercase());
            } else {
                n.push(c);
            }
        });
    });

    n
}

enum AnimateOptEnum {
    Ignore(Span),
    Expand(Span, Punctuated<Field, Comma>),
}

impl Parse for AnimateOptEnum {
    fn parse(input: ParseStream) -> ParserResult<Self> {
        syn::custom_keyword!(ignore);
        syn::custom_keyword!(fields);

        if let Some(k) = input.parse::<Option<ignore>>()? {
            Ok(AnimateOptEnum::Ignore(k.span))
        } else if let Some(k) = input.parse::<Option<fields>>()? {
            let content;
            syn::parenthesized!(content in input);
            Ok(AnimateOptEnum::Expand(
                k.span,
                content.parse_terminated(Field::parse_named)?,
            ))
        } else {
            Err(ParseError::new(input.span(), "invalid attribute format"))
        }
    }
}

#[derive(Default)]
pub struct AnimateOpt {
    pub ignore: bool,
    pub fields: Option<Punctuated<Field, Comma>>,
}

pub fn parse_animate_options(attr: &Attribute) -> ParserResult<AnimateOpt> {
    let mut opt = AnimateOpt::default();
    for attr in attr.parse_args_with(|input: ParseStream| {
        input.parse_terminated::<AnimateOptEnum, Comma>(AnimateOptEnum::parse)
    })? {
        match attr {
            AnimateOptEnum::Ignore(_) => {
                opt.ignore = true;
            }
            AnimateOptEnum::Expand(span, fields) => {
                if opt.fields.is_some() {
                    Err(ParseError::new(span, "redefined `expand` attribute"))?;
                }
                opt.fields = Some(fields);
            }
        }
    }
    Ok(opt)
}
