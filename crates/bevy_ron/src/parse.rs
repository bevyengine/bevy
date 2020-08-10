use std::{
    char::from_u32 as char_from_u32,
    fmt::{Display, Formatter, Result as FmtResult},
    str::{from_utf8, from_utf8_unchecked, FromStr},
};

use crate::{
    error::{Error, ErrorCode, Result},
    extensions::Extensions,
};

const DIGITS: &[u8] = b"0123456789ABCDEFabcdef_";
const FLOAT_CHARS: &[u8] = b"0123456789.+-eE";
const IDENT_FIRST: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_";
const IDENT_CHAR: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_0123456789";
const WHITE_SPACE: &[u8] = b"\n\t\r ";

#[derive(Clone, Debug, PartialEq)]
pub enum AnyNum {
    F32(f32),
    F64(f64),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
}

#[derive(Clone, Copy, Debug)]
pub struct Bytes<'a> {
    /// Bits set according to `Extension` enum.
    pub exts: Extensions,
    bytes: &'a [u8],
    column: usize,
    line: usize,
}

impl<'a> Bytes<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        let mut b = Bytes {
            bytes,
            column: 1,
            exts: Extensions::empty(),
            line: 1,
        };

        b.skip_ws()?;
        // Loop over all extensions attributes
        loop {
            let attribute = b.extensions()?;

            if attribute.is_empty() {
                break;
            }

            b.exts |= attribute;
            b.skip_ws()?;
        }

        Ok(b)
    }

    pub fn advance(&mut self, bytes: usize) -> Result<()> {
        for _ in 0..bytes {
            self.advance_single()?;
        }

        Ok(())
    }

    pub fn advance_single(&mut self) -> Result<()> {
        if self.peek_or_eof()? == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.bytes = &self.bytes[1..];

        Ok(())
    }

    fn any_integer<T: Num>(&mut self, sign: i8) -> Result<T> {
        let base = if self.peek() == Some(b'0') {
            match self.bytes.get(1).cloned() {
                Some(b'x') => 16,
                Some(b'b') => 2,
                Some(b'o') => 8,
                _ => 10,
            }
        } else {
            10
        };

        if base != 10 {
            // If we have `0x45A` for example,
            // cut it to `45A`.
            let _ = self.advance(2);
        }

        let num_bytes = self.next_bytes_contained_in(DIGITS);

        if num_bytes == 0 {
            return self.err(ErrorCode::ExpectedInteger);
        }

        let s = unsafe { from_utf8_unchecked(&self.bytes[0..num_bytes]) };

        if s.as_bytes()[0] == b'_' {
            return self.err(ErrorCode::UnderscoreAtBeginning);
        }

        fn calc_num<T: Num>(
            bytes: &Bytes,
            s: &str,
            base: u8,
            mut f: impl FnMut(&mut T, u8) -> bool,
        ) -> Result<T> {
            let mut num_acc = T::from_u8(0);

            for &byte in s.as_bytes() {
                if byte == b'_' {
                    continue;
                }

                if num_acc.checked_mul_ext(base) {
                    return bytes.err(ErrorCode::IntegerOutOfBounds);
                }

                let digit = bytes.decode_hex(byte)?;

                if digit >= base {
                    return bytes.err(ErrorCode::ExpectedInteger);
                }

                if f(&mut num_acc, digit) {
                    return bytes.err(ErrorCode::IntegerOutOfBounds);
                }
            }

            Ok(num_acc)
        };

        let res = if sign > 0 {
            calc_num(&*self, s, base, T::checked_add_ext)
        } else {
            calc_num(&*self, s, base, T::checked_sub_ext)
        };

        let _ = self.advance(num_bytes);

        res
    }

    pub fn any_num(&mut self) -> Result<AnyNum> {
        // We are not doing float comparisons here in the traditional sense.
        // Instead, this code checks if a f64 fits inside an f32.
        #[allow(clippy::float_cmp)]
        fn any_float(f: f64) -> Result<AnyNum> {
            if f == f as f32 as f64 {
                Ok(AnyNum::F32(f as f32))
            } else {
                Ok(AnyNum::F64(f))
            }
        }

        let bytes_backup = self.bytes;

        let first_byte = self.peek_or_eof()?;
        let is_signed = first_byte == b'-' || first_byte == b'+';
        let is_float = self.next_bytes_is_float();

        if is_float {
            let f = self.float::<f64>()?;

            any_float(f)
        } else {
            let max_u8 = std::u8::MAX as u128;
            let max_u16 = std::u16::MAX as u128;
            let max_u32 = std::u32::MAX as u128;
            let max_u64 = std::u64::MAX as u128;

            let min_i8 = std::i8::MIN as i128;
            let max_i8 = std::i8::MAX as i128;
            let min_i16 = std::i16::MIN as i128;
            let max_i16 = std::i16::MAX as i128;
            let min_i32 = std::i32::MIN as i128;
            let max_i32 = std::i32::MAX as i128;
            let min_i64 = std::i64::MIN as i128;
            let max_i64 = std::i64::MAX as i128;

            if is_signed {
                match self.signed_integer::<i128>() {
                    Ok(x) => {
                        if x >= min_i8 && x <= max_i8 {
                            Ok(AnyNum::I8(x as i8))
                        } else if x >= min_i16 && x <= max_i16 {
                            Ok(AnyNum::I16(x as i16))
                        } else if x >= min_i32 && x <= max_i32 {
                            Ok(AnyNum::I32(x as i32))
                        } else if x >= min_i64 && x <= max_i64 {
                            Ok(AnyNum::I64(x as i64))
                        } else {
                            Ok(AnyNum::I128(x))
                        }
                    }
                    Err(_) => {
                        self.bytes = bytes_backup;

                        any_float(self.float::<f64>()?)
                    }
                }
            } else {
                match self.unsigned_integer::<u128>() {
                    Ok(x) => {
                        if x <= max_u8 {
                            Ok(AnyNum::U8(x as u8))
                        } else if x <= max_u16 {
                            Ok(AnyNum::U16(x as u16))
                        } else if x <= max_u32 {
                            Ok(AnyNum::U32(x as u32))
                        } else if x <= max_u64 {
                            Ok(AnyNum::U64(x as u64))
                        } else {
                            Ok(AnyNum::U128(x))
                        }
                    }
                    Err(_) => {
                        self.bytes = bytes_backup;

                        any_float(self.float::<f64>()?)
                    }
                }
            }
        }
    }

    pub fn bool(&mut self) -> Result<bool> {
        if self.consume("true") {
            Ok(true)
        } else if self.consume("false") {
            Ok(false)
        } else {
            self.err(ErrorCode::ExpectedBoolean)
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn char(&mut self) -> Result<char> {
        if !self.consume("'") {
            return self.err(ErrorCode::ExpectedChar);
        }

        let c = self.peek_or_eof()?;

        let c = if c == b'\\' {
            let _ = self.advance(1);

            self.parse_escape()?
        } else {
            // Check where the end of the char (') is and try to
            // interpret the rest as UTF-8

            let max = self.bytes.len().min(5);
            let pos: usize = self.bytes[..max]
                .iter()
                .position(|&x| x == b'\'')
                .ok_or_else(|| self.error(ErrorCode::ExpectedChar))?;
            let s = from_utf8(&self.bytes[0..pos]).map_err(|e| self.error(e.into()))?;
            let mut chars = s.chars();

            let first = chars
                .next()
                .ok_or_else(|| self.error(ErrorCode::ExpectedChar))?;
            if chars.next().is_some() {
                return self.err(ErrorCode::ExpectedChar);
            }

            let _ = self.advance(pos);

            first
        };

        if !self.consume("'") {
            return self.err(ErrorCode::ExpectedChar);
        }

        Ok(c)
    }

    pub fn comma(&mut self) -> Result<bool> {
        self.skip_ws()?;

        if self.consume(",") {
            self.skip_ws()?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Only returns true if the char after `ident` cannot belong
    /// to an identifier.
    pub fn check_ident(&mut self, ident: &str) -> bool {
        self.test_for(ident) && !self.check_ident_char(ident.len())
    }

    fn check_ident_char(&self, index: usize) -> bool {
        self.bytes
            .get(index)
            .map_or(false, |b| IDENT_CHAR.contains(b))
    }

    /// Should only be used on a working copy
    pub fn check_tuple_struct(mut self) -> Result<bool> {
        if self.identifier().is_err() {
            // if there's no field ident, this is a tuple struct
            return Ok(true);
        }

        self.skip_ws()?;

        // if there is no colon after the ident, this can only be a unit struct
        self.eat_byte().map(|c| c != b':')
    }

    /// Only returns true if the char after `ident` cannot belong
    /// to an identifier.
    pub fn consume_ident(&mut self, ident: &str) -> bool {
        if self.check_ident(ident) {
            let _ = self.advance(ident.len());

            true
        } else {
            false
        }
    }

    pub fn consume(&mut self, s: &str) -> bool {
        if self.test_for(s) {
            let _ = self.advance(s.len());

            true
        } else {
            false
        }
    }

    fn consume_all(&mut self, all: &[&str]) -> Result<bool> {
        all.iter()
            .map(|elem| {
                if self.consume(elem) {
                    self.skip_ws()?;

                    Ok(true)
                } else {
                    Ok(false)
                }
            })
            .fold(Ok(true), |acc, x| acc.and_then(|val| x.map(|x| x && val)))
    }

    pub fn eat_byte(&mut self) -> Result<u8> {
        let peek = self.peek_or_eof()?;
        let _ = self.advance_single();

        Ok(peek)
    }

    pub fn err<T>(&self, kind: ErrorCode) -> Result<T> {
        Err(self.error(kind))
    }

    pub fn error(&self, kind: ErrorCode) -> Error {
        Error {
            code: kind,
            position: Position {
                line: self.line,
                col: self.column,
            },
        }
    }

    pub fn expect_byte(&mut self, byte: u8, error: ErrorCode) -> Result<()> {
        self.eat_byte()
            .and_then(|b| if b == byte { Ok(()) } else { self.err(error) })
    }

    /// Returns the extensions bit mask.
    fn extensions(&mut self) -> Result<Extensions> {
        if self.peek() != Some(b'#') {
            return Ok(Extensions::empty());
        }

        if !self.consume_all(&["#", "!", "[", "enable", "("])? {
            return self.err(ErrorCode::ExpectedAttribute);
        }

        self.skip_ws()?;
        let mut extensions = Extensions::empty();

        loop {
            let ident = self.identifier()?;
            let extension = Extensions::from_ident(ident).ok_or_else(|| {
                self.error(ErrorCode::NoSuchExtension(
                    from_utf8(ident).unwrap().to_owned(),
                ))
            })?;

            extensions |= extension;

            let comma = self.comma()?;

            // If we have no comma but another item, return an error
            if !comma && self.check_ident_char(0) {
                return self.err(ErrorCode::ExpectedComma);
            }

            // If there's no comma, assume the list ended.
            // If there is, it might be a trailing one, thus we only
            // continue the loop if we get an ident char.
            if !comma || !self.check_ident_char(0) {
                break;
            }
        }

        self.skip_ws()?;

        if self.consume_all(&[")", "]"])? {
            Ok(extensions)
        } else {
            Err(self.error(ErrorCode::ExpectedAttributeEnd))
        }
    }

    pub fn float<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        for literal in &["inf", "-inf", "NaN"] {
            if self.consume_ident(literal) {
                return FromStr::from_str(literal).map_err(|_| unreachable!()); // must not fail
            }
        }

        let num_bytes = self.next_bytes_contained_in(FLOAT_CHARS);

        let s = unsafe { from_utf8_unchecked(&self.bytes[0..num_bytes]) };
        let res = FromStr::from_str(s).map_err(|_| self.error(ErrorCode::ExpectedFloat));

        let _ = self.advance(num_bytes);

        res
    }

    pub fn identifier(&mut self) -> Result<&'a [u8]> {
        let bytes = self.identifier_len()?;
        let ident = &self.bytes[..bytes];
        let _ = self.advance(bytes);

        Ok(ident)
    }

    pub fn identifier_len(&self) -> Result<usize> {
        let next = self.peek_or_eof()?;
        if IDENT_FIRST.contains(&next) {
            // If the next two bytes signify the start of a raw string literal,
            // return an error.
            if next == b'r' {
                let second = self
                    .bytes
                    .get(1)
                    .ok_or_else(|| self.error(ErrorCode::Eof))?;
                if *second == b'"' || *second == b'#' {
                    return self.err(ErrorCode::ExpectedIdentifier);
                }
            }

            let bytes = self.next_bytes_contained_in(IDENT_CHAR);

            Ok(bytes)
        } else {
            self.err(ErrorCode::ExpectedIdentifier)
        }
    }

    pub fn next_bytes_contained_in(&self, allowed: &[u8]) -> usize {
        self.bytes
            .iter()
            .take_while(|b| allowed.contains(b))
            .count()
    }

    pub fn next_bytes_is_float(&self) -> bool {
        if let Some(byte) = self.peek() {
            let skip = match byte {
                b'+' | b'-' => 1,
                _ => 0,
            };
            let flen = self
                .bytes
                .iter()
                .skip(skip)
                .take_while(|b| FLOAT_CHARS.contains(b))
                .count();
            let ilen = self
                .bytes
                .iter()
                .skip(skip)
                .take_while(|b| DIGITS.contains(b))
                .count();
            flen > ilen
        } else {
            false
        }
    }

    pub fn skip_ws(&mut self) -> Result<()> {
        while self.peek().map_or(false, |c| WHITE_SPACE.contains(&c)) {
            let _ = self.advance_single();
        }

        if self.skip_comment()? {
            self.skip_ws()?;
        }

        Ok(())
    }

    pub fn peek(&self) -> Option<u8> {
        self.bytes.get(0).cloned()
    }

    pub fn peek_or_eof(&self) -> Result<u8> {
        self.bytes
            .get(0)
            .cloned()
            .ok_or_else(|| self.error(ErrorCode::Eof))
    }

    pub fn signed_integer<T>(&mut self) -> Result<T>
    where
        T: Num,
    {
        match self.peek_or_eof()? {
            b'+' => {
                let _ = self.advance_single();

                self.any_integer(1)
            }
            b'-' => {
                let _ = self.advance_single();

                self.any_integer(-1)
            }
            _ => self.any_integer(1),
        }
    }

    pub fn string(&mut self) -> Result<ParsedStr<'a>> {
        if self.consume("\"") {
            self.escaped_string()
        } else if self.consume("r") {
            self.raw_string()
        } else {
            self.err(ErrorCode::ExpectedString)
        }
    }

    fn escaped_string(&mut self) -> Result<ParsedStr<'a>> {
        use std::iter::repeat;

        let (i, end_or_escape) = self
            .bytes
            .iter()
            .enumerate()
            .find(|&(_, &b)| b == b'\\' || b == b'"')
            .ok_or_else(|| self.error(ErrorCode::ExpectedStringEnd))?;

        if *end_or_escape == b'"' {
            let s = from_utf8(&self.bytes[..i]).map_err(|e| self.error(e.into()))?;

            // Advance by the number of bytes of the string
            // + 1 for the `"`.
            let _ = self.advance(i + 1);

            Ok(ParsedStr::Slice(s))
        } else {
            let mut i = i;
            let mut s: Vec<_> = self.bytes[..i].to_vec();

            loop {
                let _ = self.advance(i + 1);
                let character = self.parse_escape()?;
                match character.len_utf8() {
                    1 => s.push(character as u8),
                    len => {
                        let start = s.len();
                        s.extend(repeat(0).take(len));
                        character.encode_utf8(&mut s[start..]);
                    }
                }

                let (new_i, end_or_escape) = self
                    .bytes
                    .iter()
                    .enumerate()
                    .find(|&(_, &b)| b == b'\\' || b == b'"')
                    .ok_or(ErrorCode::Eof)
                    .map_err(|e| self.error(e))?;

                i = new_i;
                s.extend_from_slice(&self.bytes[..i]);

                if *end_or_escape == b'"' {
                    let _ = self.advance(i + 1);

                    let s = String::from_utf8(s).map_err(|e| self.error(e.into()))?;
                    break Ok(ParsedStr::Allocated(s));
                }
            }
        }
    }

    fn raw_string(&mut self) -> Result<ParsedStr<'a>> {
        let num_hashes = self.bytes.iter().take_while(|&&b| b == b'#').count();
        let hashes = &self.bytes[..num_hashes];
        let _ = self.advance(num_hashes);

        if !self.consume("\"") {
            return self.err(ErrorCode::ExpectedString);
        }

        let ending = [&[b'"'], hashes].concat();
        let i = self
            .bytes
            .windows(num_hashes + 1)
            .position(|window| window == ending.as_slice())
            .ok_or_else(|| self.error(ErrorCode::ExpectedStringEnd))?;

        let s = from_utf8(&self.bytes[..i]).map_err(|e| self.error(e.into()))?;

        // Advance by the number of bytes of the string
        // + `num_hashes` + 1 for the `"`.
        let _ = self.advance(i + num_hashes + 1);

        Ok(ParsedStr::Slice(s))
    }

    fn test_for(&self, s: &str) -> bool {
        s.bytes()
            .enumerate()
            .all(|(i, b)| self.bytes.get(i).map_or(false, |t| *t == b))
    }

    pub fn unsigned_integer<T: Num>(&mut self) -> Result<T> {
        self.any_integer(1)
    }

    fn decode_ascii_escape(&mut self) -> Result<u8> {
        let mut n = 0;
        for _ in 0..2 {
            n <<= 4;
            let byte = self.eat_byte()?;
            let decoded = self.decode_hex(byte)?;
            n |= decoded;
        }

        Ok(n)
    }

    #[inline]
    fn decode_hex(&self, c: u8) -> Result<u8> {
        match c {
            c @ b'0'..=b'9' => Ok(c - b'0'),
            c @ b'a'..=b'f' => Ok(10 + c - b'a'),
            c @ b'A'..=b'F' => Ok(10 + c - b'A'),
            _ => self.err(ErrorCode::InvalidEscape("Non-hex digit found")),
        }
    }

    fn parse_escape(&mut self) -> Result<char> {
        let c = match self.eat_byte()? {
            b'\'' => '\'',
            b'"' => '"',
            b'\\' => '\\',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'x' => self.decode_ascii_escape()? as char,
            b'u' => {
                self.expect_byte(b'{', ErrorCode::InvalidEscape("Missing {"))?;

                let mut bytes: u32 = 0;
                let mut num_digits = 0;

                while num_digits < 6 {
                    let byte = self.peek_or_eof()?;

                    if byte == b'}' {
                        break;
                    } else {
                        self.advance_single()?;
                    }

                    let byte = self.decode_hex(byte)?;
                    bytes <<= 4;
                    bytes |= byte as u32;

                    num_digits += 1;
                }

                if num_digits == 0 {
                    return self.err(ErrorCode::InvalidEscape(
                        "Expected 1-6 digits, got 0 digits",
                    ));
                }

                self.expect_byte(b'}', ErrorCode::InvalidEscape("No } at the end"))?;
                char_from_u32(bytes)
                    .ok_or_else(|| self.error(ErrorCode::InvalidEscape("Not a valid char")))?
            }
            _ => {
                return self.err(ErrorCode::InvalidEscape("Unknown escape character"));
            }
        };

        Ok(c)
    }

    fn skip_comment(&mut self) -> Result<bool> {
        if self.consume("/") {
            match self.eat_byte()? {
                b'/' => {
                    let bytes = self.bytes.iter().take_while(|&&b| b != b'\n').count();

                    let _ = self.advance(bytes);
                }
                b'*' => {
                    let mut level = 1;

                    while level > 0 {
                        let bytes = self
                            .bytes
                            .iter()
                            .take_while(|&&b| b != b'/' && b != b'*')
                            .count();

                        if self.bytes.is_empty() {
                            return self.err(ErrorCode::UnclosedBlockComment);
                        }

                        let _ = self.advance(bytes);

                        // check whether / or * and take action
                        if self.consume("/*") {
                            level += 1;
                        } else if self.consume("*/") {
                            level -= 1;
                        } else {
                            self.eat_byte()
                                .map_err(|_| self.error(ErrorCode::UnclosedBlockComment))?;
                        }
                    }
                }
                b => return self.err(ErrorCode::UnexpectedByte(b as char)),
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub trait Num {
    fn from_u8(x: u8) -> Self;

    /// Returns `true` on overflow
    fn checked_mul_ext(&mut self, x: u8) -> bool;

    /// Returns `true` on overflow
    fn checked_add_ext(&mut self, x: u8) -> bool;

    /// Returns `true` on overflow
    fn checked_sub_ext(&mut self, x: u8) -> bool;
}

macro_rules! impl_num {
    ($ty:ident) => {
        impl Num for $ty {
            fn from_u8(x: u8) -> Self {
                x as $ty
            }

            fn checked_mul_ext(&mut self, x: u8) -> bool {
                match self.checked_mul(Self::from_u8(x)) {
                    Some(n) => {
                        *self = n;
                        false
                    }
                    None => true,
                }
            }

            fn checked_add_ext(&mut self, x: u8) -> bool {
                match self.checked_add(Self::from_u8(x)) {
                    Some(n) => {
                        *self = n;
                        false
                    }
                    None => true,
                }
            }

            fn checked_sub_ext(&mut self, x: u8) -> bool {
                match self.checked_sub(Self::from_u8(x)) {
                    Some(n) => {
                        *self = n;
                        false
                    }
                    None => true,
                }
            }
        }
    };
    ($($tys:ident)*) => {
        $( impl_num!($tys); )*
    };
}

impl_num!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128);

#[derive(Clone, Debug)]
pub enum ParsedStr<'a> {
    Allocated(String),
    Slice(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_x10() {
        let mut bytes = Bytes::new(b"10").unwrap();
        assert_eq!(bytes.decode_ascii_escape(), Ok(0x10));
    }
}
