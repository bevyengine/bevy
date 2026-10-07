//! Support for parsing the BSN file format
extern crate alloc;

/// Lexing and Tokens
pub mod lex;

/// BSN Parsing logic
pub mod parse;

/// A parseable stream of tokens.
pub mod parse_stream;

/// Span functionality, for referencing "spans" of BSN source strings.
pub mod span;

/// BSN Types. This is the Abstract Syntax Tree (AST).
pub mod types;
