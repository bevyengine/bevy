use crate::span::Span;
use nom::{
    branch::alt,
    bytes::{
        complete::{tag, take_while, take_while1},
        tag_no_case,
    },
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, one_of},
    combinator::{opt, recognize},
    multi::many0_count,
    sequence::{delimited, pair},
    IResult, Input, Parser,
};
use std::ops::Deref;
use thiserror::Error;

/// A BSN token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Boolean value: true, false.
    Bool(bool),
    /// Identifier: `Foo`, `bar`, `Baz12`, `FooBar`
    Ident(String),
    /// String literal: "hello world"
    StringLit(String),
    /// Integer: 1, 10, -10
    // TODO: distinguish UInt IInt, optional explicit size
    Int(i128),
    /// Floating point: 0.1, .1, 10.1
    Float(f64),
    /// Left bracket: [
    LBracket,
    /// Right bracket: ]
    RBracket,
    /// Left Paren: (
    LParen,
    /// Right Paren: )
    RParen,
    /// Left Brace: {
    LBrace,
    /// Right Brace: }
    RBrace,
    /// Comma: ,
    Comma,
    /// Double Colon: ::
    DoubleColon,
    /// Colon: :
    Colon,
    /// Double Minus: --
    DoubleMinus,
    /// Minus: -
    Minus,
    /// Hash: #
    Hash,
    /// At: @
    At,
}

impl Token {
    fn spanned<'a>(self, span: Span<'a>) -> SpannedToken<'a> {
        SpannedToken { span, token: self }
    }
}

/// A [`Token`] coupled with the [`Span`] that produced it.
#[derive(Debug, Clone)]
pub struct SpannedToken<'a> {
    /// A [`Token`].
    pub token: Token,
    /// The [`Span`] that produced the [`Token`].
    pub span: Span<'a>,
}

impl<'a> Deref for SpannedToken<'a> {
    type Target = Token;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

fn lex_token(input: Span) -> IResult<Span, SpannedToken> {
    alt((
        lex_true,
        lex_false,
        lex_ident,
        lex_string,
        lex_l_brace,
        lex_r_brace,
        lex_l_paren,
        lex_r_paren,
        lex_l_bracket,
        lex_r_bracket,
        lex_comma,
        lex_double_colon,
        lex_colon,
        lex_double_minus,
        lex_minus,
        lex_hash,
        lex_at,
        lex_float,
        lex_int,
    ))
    .parse(input)
}

fn lex_true(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("true")(input)?;
    Ok((rest, Token::Bool(true).spanned(value)))
}

fn lex_false(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("false")(input)?;
    Ok((rest, Token::Bool(false).spanned(value)))
}

fn lex_ident(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;
    Ok((rest, Token::Ident(value.fragment.to_owned()).spanned(value)))
}

fn lex_l_bracket(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("[")(input)?;
    Ok((rest, Token::LBracket.spanned(value)))
}

fn lex_r_bracket(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("]")(input)?;
    Ok((rest, Token::RBracket.spanned(value)))
}

fn lex_l_paren(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("(")(input)?;
    Ok((rest, Token::LParen.spanned(value)))
}

fn lex_r_paren(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag(")")(input)?;
    Ok((rest, Token::RParen.spanned(value)))
}

fn lex_l_brace(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("{")(input)?;
    Ok((rest, Token::LBrace.spanned(value)))
}

fn lex_r_brace(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("}")(input)?;
    Ok((rest, Token::RBrace.spanned(value)))
}

fn lex_comma(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag(",")(input)?;
    Ok((rest, Token::Comma.spanned(value)))
}

fn lex_colon(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag(":")(input)?;
    Ok((rest, Token::Colon.spanned(value)))
}

fn lex_double_colon(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("::")(input)?;
    Ok((rest, Token::DoubleColon.spanned(value)))
}

fn lex_minus(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("-")(input)?;
    Ok((rest, Token::Minus.spanned(value)))
}

fn lex_double_minus(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("--")(input)?;
    Ok((rest, Token::DoubleMinus.spanned(value)))
}

fn lex_hash(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("#")(input)?;
    Ok((rest, Token::Hash.spanned(value)))
}

fn lex_at(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, value) = tag("@")(input)?;
    Ok((rest, Token::At.spanned(value)))
}

fn lex_int(input: Span) -> IResult<Span, SpannedToken> {
    let start = input.clone();
    let (rest, (sign, radix_prefix, digits)) = (
        opt(alt((tag("+"), tag("-")))),
        opt(tag_no_case("0x")),
        alt((
            take_while1(|c: char| c.is_ascii_hexdigit()),
            take_while1(|c: char| c.is_ascii_digit()),
        )),
    )
        .parse(input.clone())?;

    let is_hex = radix_prefix.is_some();
    let is_negative = sign.map(|s| s.fragment) == Some("-");
    let radix = if is_hex { 16 } else { 10 };

    // Make sure decimal numbers have no hex digits.
    if !is_hex
        && digits
            .fragment
            .chars()
            .any(|c| c.is_ascii_hexdigit() && !c.is_ascii_digit())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }

    let int_span = Input::take(&start, (digits.offset - start.offset) + digits.len());
    match i128::from_str_radix(digits.fragment, radix) {
        Ok(integer) if is_negative => Ok((rest, Token::Int(-integer).spanned(int_span))),
        Ok(integer) => Ok((rest, Token::Int(integer).spanned(int_span))),
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        ))),
    }
}

fn lex_float(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, raw) = recognize((
        opt(one_of("+-")),
        opt(digit1),
        char('.'),
        opt(digit1),
        opt((one_of("eE"), opt(one_of("+-")), digit1)),
    ))
    .parse(input.clone())?;

    // `.` isn't a valid number.
    if raw.fragment == "." {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Float,
        )));
    }

    let value: f64 = raw.fragment.parse().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float))
    })?;

    Ok((rest, Token::Float(value).spanned(raw)))
}

fn lex_whitespace(input: Span) -> IResult<Span, ()> {
    let (rest, _) = multispace0(input)?;
    Ok((rest, ()))
}

fn lex_string(input: Span) -> IResult<Span, SpannedToken> {
    let (rest, (value_span, value)) = delimited(char('"'), string_body, char('"')).parse(input)?;
    return Ok((rest, Token::StringLit(value).spanned(value_span)));

    fn string_body(mut input: Span) -> IResult<Span, (Span, String)> {
        let mut out = String::new();
        let start = input.clone();
        loop {
            let (rest, chunk) = take_while(|c: char| c != '\\' && c != '"')(input)?;
            out.push_str(chunk.fragment);
            input = rest;

            if input.starts_with('"') || input.is_empty() {
                break;
            }

            let (rest, _) = char('\\')(input)?;
            if rest.is_empty() {
                out.push('\\');
                input = rest;
                break;
            }
            let esc_char = rest.chars().next().unwrap();
            let rest = rest.take_from(esc_char.len_utf8());
            match esc_char {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '0' => out.push('\0'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'x' => {
                    if rest.len() >= 2 {
                        let hex = &rest[..2];
                        if let Ok(byte) = u8::from_str_radix(hex, 16) {
                            out.push(byte as char);
                            input = rest.take_from(2);
                            continue;
                        }
                    }
                    out.push('\\');
                    out.push('x');
                    input = rest;
                    continue;
                }
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
            input = rest;
        }

        let span_size = input.offset - start.offset;
        Ok((input, (Input::take(&start, span_size), out)))
    }
}

/// An error that occurs when lexing BSN.
#[derive(Error, Debug)]
pub enum LexError {
    /// Occurs when an unexpected character is encountered during lexing.
    #[error("Unexpected character: {0}")]
    UnexpectedChar(char),
}

impl<'a> Iterator for Span<'a> {
    type Item = Result<SpannedToken<'a>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (rest, _) = lex_whitespace(self.clone()).unwrap();
        *self = rest;
        if self.fragment.is_empty() {
            return None;
        }

        let start = self.clone();
        match lex_token(start.clone()) {
            Ok((rest, token)) => {
                *self = rest;
                Some(Ok(token))
            }
            Err(_) => Some(Err(LexError::UnexpectedChar(
                start.fragment.chars().next().unwrap(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lex::{Span, Token};

    #[test]
    fn lex() {
        let results = Span::from(
            r#"true false Foo foo_bar1 Foo2Bar { }
            ( ) [ ] :: : , # @ 1 10 0.1 1.0 10.1
            "hello" "a \" \t"
            - --
            "#,
        )
        .lex_all()
        .unwrap();
        assert_eq!(
            results,
            vec![
                Token::Bool(true),
                Token::Bool(false),
                Token::Ident("Foo".to_string()),
                Token::Ident("foo_bar1".to_string()),
                Token::Ident("Foo2Bar".to_string()),
                Token::LBrace,
                Token::RBrace,
                Token::LParen,
                Token::RParen,
                Token::LBracket,
                Token::RBracket,
                Token::DoubleColon,
                Token::Colon,
                Token::Comma,
                Token::Hash,
                Token::At,
                Token::Int(1),
                Token::Int(10),
                Token::Float(0.1),
                Token::Float(1.0),
                Token::Float(10.1),
                Token::StringLit("hello".to_string()),
                Token::StringLit("a \" \t".to_string()),
                Token::Minus,
                Token::DoubleMinus,
            ]
        );
    }
}
