use nom::{IResult, Parser as _};

#[derive(Clone, PartialEq, Debug)]
pub enum Token {
    Ident(String),
    StringLit(String),
    IntLit(i128),
    FloatLit(f64),
    BoolLit(bool),
    LBracket,
    RBracket,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    DoubleColon,
    Colon,
    Hash,
    At,
}

#[derive(Debug)]
pub enum Error {
    UnexpectedChar(char),
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Lexer<'a> {
        Lexer { input, pos: 0 }
    }
}

fn lex_c_comment(input: &str) -> IResult<&str, ()> {
    nom::combinator::value(
        (),
        nom::sequence::delimited(
            nom::bytes::complete::tag("/*"),
            nom::multi::many0(nom::branch::alt((
                nom::combinator::map(lex_c_comment, |_| ' '),
                nom::character::complete::none_of("*"),
                nom::combinator::map(
                    nom::sequence::terminated(
                        nom::bytes::complete::tag("*"),
                        nom::combinator::not(nom::bytes::complete::tag("/")),
                    ),
                    |_| '*',
                ),
            ))),
            nom::bytes::complete::tag("*/"),
        ),
    )
    .parse(input)
}

fn lex_cpp_comment(input: &str) -> IResult<&str, ()> {
    nom::combinator::value(
        (),
        (
            nom::bytes::complete::tag("//"),
            nom::multi::many0(nom::character::complete::none_of("\n")),
            nom::character::complete::newline,
        ),
    )
    .parse(input)
}

fn lex_ignorable(input: &str) -> IResult<&str, ()> {
    nom::combinator::value(
        (),
        nom::multi::many0(nom::branch::alt((
            nom::bytes::complete::take_while1(|c: char| c.is_ascii_whitespace()),
            nom::combinator::map(lex_c_comment, |_| ""),
            nom::combinator::map(lex_cpp_comment, |_| ""),
        ))),
    )
    .parse(input)
}

fn lex_l_paren(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::LParen, nom::character::complete::char('(')).parse(input)
}
fn lex_r_paren(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::RParen, nom::character::complete::char(')')).parse(input)
}
fn lex_l_bracket(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::LBracket, nom::character::complete::char('[')).parse(input)
}
fn lex_r_bracket(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::RBracket, nom::character::complete::char(']')).parse(input)
}
fn lex_l_brace(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::LBrace, nom::character::complete::char('{')).parse(input)
}
fn lex_r_brace(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::RBrace, nom::character::complete::char('}')).parse(input)
}
fn lex_hash(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::Hash, nom::character::complete::char('#')).parse(input)
}
fn lex_double_colon(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::DoubleColon, nom::bytes::complete::tag("::")).parse(input)
}
fn lex_colon(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::Colon, nom::character::complete::char(':')).parse(input)
}
fn lex_comma(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::Comma, nom::character::complete::char(',')).parse(input)
}
fn lex_at(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::At, nom::character::complete::char('@')).parse(input)
}

fn lex_false(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::BoolLit(false), nom::bytes::complete::tag("false")).parse(input)
}
fn lex_true(input: &str) -> IResult<&str, Token> {
    nom::combinator::value(Token::BoolLit(true), nom::bytes::complete::tag("true")).parse(input)
}

fn lex_ident(ident: &str) -> IResult<&str, Token> {
    let (rest, s) = nom::combinator::recognize(nom::sequence::pair(
        nom::bytes::complete::take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
        nom::bytes::complete::take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
    ))
    .parse(ident)?;
    Ok((rest, Token::Ident(s.to_owned())))
}

fn lex_string(input: &str) -> IResult<&str, Token> {
    let (rest, s) = nom::sequence::delimited(
        nom::character::complete::char('"'),
        string_body,
        nom::character::complete::char('"'),
    )
    .parse(input)?;
    return Ok((rest, Token::StringLit(s)));

    fn string_body(mut input: &str) -> IResult<&str, String> {
        let mut out = String::new();

        loop {
            let (rest, chunk) =
                nom::bytes::complete::take_while(|c: char| c != '\\' && c != '"')(input)?;
            out.push_str(chunk);
            input = rest;

            if input.starts_with('"') || input.is_empty() {
                break;
            }

            let (rest, _) = nom::character::complete::char('\\')(input)?;
            if rest.is_empty() {
                out.push('\\');
                input = rest;
                break;
            }
            let esc_char = rest.chars().next().unwrap();
            let rest = &rest[esc_char.len_utf8()..];
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
                            input = &rest[2..];
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

        Ok((input, out))
    }
}

fn lex_int(input: &str) -> IResult<&str, Token> {
    let (rest, (sign, radix_prefix, digits)) = nom::sequence::tuple((
        nom::combinator::opt(nom::branch::alt((
            nom::bytes::complete::tag("+"),
            nom::bytes::complete::tag("-"),
        ))),
        nom::combinator::opt(nom::bytes::complete::tag_no_case("0x")),
        nom::branch::alt((
            nom::bytes::complete::take_while1(|c: char| c.is_ascii_hexdigit()),
            nom::bytes::complete::take_while1(|c: char| c.is_ascii_digit()),
        )),
    ))
    .parse(input)?;

    let is_hex = radix_prefix.is_some();
    let is_negative = sign == Some("-");
    let radix = if is_hex { 16 } else { 10 };

    // Make sure decimal numbers have no hex digits.
    if !is_hex
        && digits
            .chars()
            .any(|c| c.is_ascii_hexdigit() && !c.is_ascii_digit())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }

    match i128::from_str_radix(digits, radix) {
        Ok(integer) if is_negative => Ok((rest, Token::IntLit(-integer))),
        Ok(integer) => Ok((rest, Token::IntLit(integer))),
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        ))),
    }
}

fn lex_float(input: &str) -> IResult<&str, Token> {
    let (rest, raw) = nom::combinator::recognize(nom::sequence::tuple((
        nom::combinator::opt(nom::character::complete::one_of("+-")),
        nom::combinator::opt(nom::character::complete::digit1),
        nom::character::complete::char('.'),
        nom::combinator::opt(nom::character::complete::digit1),
        nom::combinator::opt(nom::sequence::tuple((
            nom::character::complete::one_of("eE"),
            nom::combinator::opt(nom::character::complete::one_of("+-")),
            nom::character::complete::digit1,
        ))),
    )))
    .parse(input)?;

    // `.` isn't a valid number.
    if raw == "." {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Float,
        )));
    }

    let value: f64 = raw.parse().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float))
    })?;

    Ok((rest, Token::FloatLit(value)))
}

fn lex_token(input: &str) -> IResult<&str, Token> {
    nom::branch::alt((
        lex_true,  // Must come before `lex_ident`.
        lex_false, // Must come before `lex_ident`.
        lex_ident,
        lex_string,
        lex_float,  // Must come before `lex_int`.
        lex_int,
        lex_l_bracket,
        lex_r_bracket,
        lex_l_paren,
        lex_r_paren,
        lex_l_brace,
        lex_r_brace,
        lex_comma,
        lex_double_colon, // Must come before `lex_colon`.
        lex_colon,
        lex_hash,
        lex_at,
    ))
    .parse(input)
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<(usize, Token, usize), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let (rest, _) = lex_ignorable(self.input).unwrap();
        self.pos += self.input.len() - rest.len();
        self.input = rest;
        if self.input.is_empty() {
            return None;
        }

        match lex_token(self.input) {
            Ok((rest, token)) => {
                let start_pos = self.pos;
                let end_pos = start_pos + self.input.len() - rest.len();
                self.input = rest;
                self.pos = end_pos;
                Some(Ok((start_pos, token, end_pos)))
            }
            Err(_) => Some(Err(Error::UnexpectedChar(
                self.input.chars().next().unwrap(),
            ))),
        }
    }
}
