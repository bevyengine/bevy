use crate::{
    lex::Token,
    parse_stream::{Parse, ParseError, ParseStream, Peek},
};

/// The root of a BSN document.
#[derive(Debug, Clone, PartialEq)]
pub struct BsnRoot(pub Bsn);

/// A BSN definition for a single entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Bsn {
    /// The entries in this BSN definition.
    pub entries: Vec<BsnEntry>,
}

/// An entry within a [`Bsn`] that defines something inside of it.
#[derive(Debug, Clone, PartialEq)]
pub enum BsnEntry {
    /// A named entity reference
    Name(Ident),
    /// An included cached scene
    CachedScene(BsnScene),
    /// A patch of a `FromTemplate` type
    FromTemplatePatch(BsnType),
    /// Related scene definitions.
    RelatedSceneList {
        /// The type path of the relationship target.
        relationship_target_path: Path,
        /// The list of related scene definitions.
        scene_list: BsnSceneList,
    },
}

/// Another BSN definition to include in the current scene.
#[derive(Debug, Clone, PartialEq)]
pub enum BsnScene {
    /// A "scene asset" path
    Asset(StringLit),
}

/// A list of BSN documents (each item in the list corresponding to a different entity), bounded by
/// brackets.
#[derive(Debug, Clone, PartialEq)]
pub struct BsnSceneList(pub BsnSceneListItems);

/// A flat list of BSN documents (each item in the list corresponding to a different entity).
#[derive(Debug, Clone, PartialEq)]
pub struct BsnSceneListItems(pub Vec<Bsn>);

/// A definition of a type in BSN. This maps to a Rust datatype.
#[derive(Debug, Clone, PartialEq)]
pub struct BsnType {
    /// The type path of the type.
    pub path: Path,
    /// The enum variant, if one exists.
    pub variant: Option<Ident>,
    /// The fields of the type.
    pub fields: BsnFields,
}

/// The fields of a [`BsnType`].
#[derive(Debug, Clone, PartialEq)]
pub enum BsnFields {
    /// Named fields: { field: "value" }
    NamedFields(Vec<BsnNamedField>),
    /// Unnamed fields: ("value1", value"2)
    UnnamedFields(Vec<BsnValue>),
    /// No fields
    Unit,
}

/// A named field. Ex: field: "value"
#[derive(Debug, Clone, PartialEq)]
pub struct BsnNamedField {
    /// The name of the field
    pub name: String,
    /// The value of the field
    pub value: BsnValue,
}

/// The BSN value type.
#[derive(Debug, Clone, PartialEq)]
pub enum BsnValue {
    /// A floating point number
    Float(f64),
    /// An integer number
    Int(i128),
    /// A boolean: true or false
    Bool(bool),
    /// A string literal
    String(String),
    /// A BSN type
    Type(BsnType),
}

/// A path to a type. This follows Rust conventions.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Whether or not this path has a leading colon.
    pub leading_colon: bool,
    /// The segments of the path.
    pub segments: Vec<PathSegment>,
}

/// A segment of a path, corresponding to a single identifier.
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    /// The identifier of the segment.
    pub ident: Ident,
}

impl BsnType {
    /// Creates a new [`BsnType`] from the given `path` and `fields`.
    pub fn new(path: impl Into<Path>, fields: impl Into<BsnFields>) -> Self {
        BsnType {
            path: path.into(),
            variant: None,
            fields: fields.into(),
        }
    }

    /// Creates a new [`BsnType`] from the given `path`
    pub fn path(path: impl Into<Path>) -> Self {
        BsnType {
            path: path.into(),
            variant: None,
            fields: BsnFields::Unit,
        }
    }
}

impl<I: Into<String>> From<I> for PathSegment {
    fn from(value: I) -> Self {
        PathSegment {
            ident: Ident(value.into()),
        }
    }
}

impl Path {
    /// Converts the `Path` to the equivalent type path (which is a canonical, Rust-like path to the type.)
    pub fn to_type_path(&self) -> String {
        // NOTE: skipping leading_colon here as type paths are normalized
        // PERF: could probably accurately reserve space here
        let mut value = String::default();
        for (index, segment) in self.segments.iter().enumerate() {
            value.push_str(&segment.ident.0);
            if index != self.segments.len() - 1 {
                value.push_str("::");
            }
        }
        value
    }
}

impl From<Vec<PathSegment>> for Path {
    fn from(segments: Vec<PathSegment>) -> Self {
        Self {
            leading_colon: false,
            segments,
        }
    }
}

impl<const LEN: usize, I: Into<String>> From<[I; LEN]> for Path {
    fn from(segments: [I; LEN]) -> Self {
        Self {
            leading_colon: false,
            segments: segments
                .into_iter()
                .map(|i| PathSegment::from(i))
                .collect::<Vec<_>>(),
        }
    }
}

impl BsnNamedField {
    /// Creates a new instance with the given `name` and `value`.
    pub fn new(name: impl Into<String>, value: BsnValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

macro_rules! impl_parse_token {
    ($token:ident) => {
        impl Parse for $token {
            fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError> {
                match input.next_token()? {
                    Some(token) => {
                        if matches!(token.token, Token::$token) {
                            Ok($token)
                        } else {
                            Err(ParseError::UnexpectedToken(token.into()))
                        }
                    }
                    None => Err(ParseError::EndOfInput),
                }
            }
        }

        impl Peek for $token {
            fn peek(input: &mut ParseStream) -> bool {
                if let Some(token) = input.peek_token() {
                    matches!(token.token, Token::$token)
                } else {
                    false
                }
            }
        }
    };
}

macro_rules! impl_parse_arg_token {
    ($token:ident) => {
        impl Parse for $token {
            fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError> {
                match input.next_token()? {
                    Some(token) => {
                        if let Token::$token(value) = token.token {
                            Ok($token(value))
                        } else {
                            Err(ParseError::UnexpectedToken(token.into()))
                        }
                    }
                    None => Err(ParseError::EndOfInput),
                }
            }
        }

        impl Peek for $token {
            fn peek(input: &mut ParseStream) -> bool {
                if let Some(token) = input.peek_token() {
                    matches!(token.token, Token::$token(_))
                } else {
                    false
                }
            }
        }
    };
}

/// Boolean value: true, false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bool(pub bool);
impl_parse_arg_token!(Bool);

/// Identifier: `Foo`, `bar`, `Baz12`, `FooBar`
// PERF: Consider borrowing this from the source input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident(pub String);
impl_parse_arg_token!(Ident);

impl From<&str> for Ident {
    fn from(value: &str) -> Self {
        Ident(value.into())
    }
}

/// String literal: "hello world"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLit(pub String);
impl_parse_arg_token!(StringLit);

/// Integer: 1, 10, -10
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Int(pub i128);
impl_parse_arg_token!(Int);

/// Floating point: 0.1, .1, 10.1
#[derive(Debug, Clone, PartialEq)]
pub struct Float(pub f64);
impl_parse_arg_token!(Float);

/// Left bracket: [
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LBracket;
impl_parse_token!(LBracket);

/// Right bracket: ]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RBracket;
impl_parse_token!(RBracket);

/// Left Paren: (
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LParen;
impl_parse_token!(LParen);

/// Right Paren: )
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RParen;
impl_parse_token!(RParen);

/// Left Brace: {
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LBrace;
impl_parse_token!(LBrace);

/// Right Brace: }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RBrace;
impl_parse_token!(RBrace);

/// Comma: ,
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comma;
impl_parse_token!(Comma);

/// Double Colon: ::
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleColon;
impl_parse_token!(DoubleColon);

/// Double Minus: --
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleMinus;
impl_parse_token!(DoubleMinus);

/// Colon: :
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colon;
impl_parse_token!(Colon);

/// Hash: #
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash;
impl_parse_token!(Hash);

/// At: @
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct At;
impl_parse_token!(At);
