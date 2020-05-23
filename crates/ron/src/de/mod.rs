/// Deserialization module.
pub use crate::error::{Error, ErrorCode, Result};
pub use crate::parse::Position;

use serde::de::{self, DeserializeSeed, Deserializer as SerdeError, Visitor};
use std::{borrow::Cow, io, str};

use self::{id::IdDeserializer, tag::TagDeserializer};
use crate::{
    extensions::Extensions,
    parse::{AnyNum, Bytes, ParsedStr},
};

mod id;
mod tag;
#[cfg(test)]
mod tests;
mod value;

/// The RON deserializer.
///
/// If you just want to simply deserialize a value,
/// you can use the `from_str` convenience function.
pub struct Deserializer<'de> {
    bytes: Bytes<'de>,
}

impl<'de> Deserializer<'de> {
    // Cannot implement trait here since output is tied to input lifetime 'de.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Result<Self> {
        Deserializer::from_bytes(input.as_bytes())
    }

    pub fn from_bytes(input: &'de [u8]) -> Result<Self> {
        Ok(Deserializer {
            bytes: Bytes::new(input)?,
        })
    }

    pub fn remainder(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes.bytes())
    }
}

/// A convenience function for reading data from a reader
/// and feeding into a deserializer.
pub fn from_reader<R, T>(mut rdr: R) -> Result<T>
where
    R: io::Read,
    T: de::DeserializeOwned,
{
    let mut bytes = Vec::new();
    rdr.read_to_end(&mut bytes)?;

    from_bytes(&bytes)
}

/// A convenience function for building a deserializer
/// and deserializing a value of type `T` from a string.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: de::Deserialize<'a>,
{
    from_bytes(s.as_bytes())
}

/// A convenience function for building a deserializer
/// and deserializing a value of type `T` from bytes.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: de::Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s)?;
    let t = T::deserialize(&mut deserializer)?;

    deserializer.end()?;

    Ok(t)
}

impl<'de> Deserializer<'de> {
    /// Check if the remaining bytes are whitespace only,
    /// otherwise return an error.
    pub fn end(&mut self) -> Result<()> {
        self.bytes.skip_ws()?;

        if self.bytes.bytes().is_empty() {
            Ok(())
        } else {
            self.bytes.err(ErrorCode::TrailingCharacters)
        }
    }

    /// Called from `deserialize_any` when a struct was detected. Decides if
    /// there is a unit, tuple or usual struct and deserializes it
    /// accordingly.
    ///
    /// This method assumes there is no identifier left.
    fn handle_any_struct<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Create a working copy
        let mut bytes = self.bytes;

        if bytes.consume("(") {
            bytes.skip_ws()?;

            if bytes.check_tuple_struct()? {
                // first argument is technically incorrect, but ignored anyway
                self.deserialize_tuple(0, visitor)
            } else {
                // first two arguments are technically incorrect, but ignored anyway
                self.deserialize_struct("", &[], visitor)
            }
        } else {
            visitor.visit_unit()
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume_ident("true") {
            return visitor.visit_bool(true);
        } else if self.bytes.consume_ident("false") {
            return visitor.visit_bool(false);
        } else if self.bytes.check_ident("Some") {
            return self.deserialize_option(visitor);
        } else if self.bytes.consume_ident("None") {
            return visitor.visit_none();
        } else if self.bytes.consume("()") {
            return visitor.visit_unit();
        } else if self.bytes.consume_ident("inf") {
            return visitor.visit_f64(std::f64::INFINITY);
        } else if self.bytes.consume_ident("-inf") {
            return visitor.visit_f64(std::f64::NEG_INFINITY);
        } else if self.bytes.consume_ident("NaN") {
            return visitor.visit_f64(std::f64::NAN);
        }

        // `identifier` does not change state if it fails
        let ident = self.bytes.identifier().ok();

        if ident.is_some() {
            self.bytes.skip_ws()?;

            return self.handle_any_struct(visitor);
        }

        match self.bytes.peek_or_eof()? {
            b'(' => self.handle_any_struct(visitor),
            b'[' => self.deserialize_seq(visitor),
            b'{' => self.deserialize_map(visitor),
            b'0'..=b'9' | b'+' | b'-' => {
                let any_num: AnyNum = self.bytes.any_num()?;

                match any_num {
                    AnyNum::F32(x) => visitor.visit_f32(x),
                    AnyNum::F64(x) => visitor.visit_f64(x),
                    AnyNum::I8(x) => visitor.visit_i8(x),
                    AnyNum::U8(x) => visitor.visit_u8(x),
                    AnyNum::I16(x) => visitor.visit_i16(x),
                    AnyNum::U16(x) => visitor.visit_u16(x),
                    AnyNum::I32(x) => visitor.visit_i32(x),
                    AnyNum::U32(x) => visitor.visit_u32(x),
                    AnyNum::I64(x) => visitor.visit_i64(x),
                    AnyNum::U64(x) => visitor.visit_u64(x),
                    AnyNum::I128(x) => visitor.visit_i128(x),
                    AnyNum::U128(x) => visitor.visit_u128(x),
                }
            }
            b'.' => self.deserialize_f64(visitor),
            b'"' | b'r' => self.deserialize_string(visitor),
            b'\'' => self.deserialize_char(visitor),
            other => self.bytes.err(ErrorCode::UnexpectedByte(other as char)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.bytes.bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.bytes.signed_integer()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.bytes.signed_integer()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.bytes.signed_integer()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.bytes.signed_integer()?)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(self.bytes.signed_integer()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(self.bytes.unsigned_integer()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.bytes.float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.bytes.float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.bytes.char()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.bytes.string()? {
            ParsedStr::Allocated(s) => visitor.visit_string(s),
            ParsedStr::Slice(s) => visitor.visit_borrowed_str(s),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let res = {
            let string = self.bytes.string()?;
            let base64_str = match string {
                ParsedStr::Allocated(ref s) => s.as_str(),
                ParsedStr::Slice(ref s) => s,
            };
            base64::decode(base64_str)
        };

        match res {
            Ok(byte_buf) => visitor.visit_byte_buf(byte_buf),
            Err(err) => self.bytes.err(ErrorCode::Base64Error(err)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("None") {
            visitor.visit_none()
        } else if self.bytes.exts.contains(Extensions::IMPLICIT_SOME) {
            visitor.visit_some(&mut *self)
        } else if self.bytes.consume("Some") && {
            self.bytes.skip_ws()?;
            self.bytes.consume("(")
        } {
            self.bytes.skip_ws()?;

            let v = visitor.visit_some(&mut *self)?;

            self.bytes.skip_ws()?;

            if self.bytes.consume(")") {
                Ok(v)
            } else {
                self.bytes.err(ErrorCode::ExpectedOptionEnd)
            }
        } else {
            self.bytes.err(ErrorCode::ExpectedOption)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("()") {
            visitor.visit_unit()
        } else {
            self.bytes.err(ErrorCode::ExpectedUnit)
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume(name) {
            visitor.visit_unit()
        } else {
            self.deserialize_unit(visitor)
        }
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.exts.contains(Extensions::UNWRAP_NEWTYPES) {
            return visitor.visit_newtype_struct(&mut *self);
        }

        self.bytes.consume(name);

        self.bytes.skip_ws()?;

        if self.bytes.consume("(") {
            self.bytes.skip_ws()?;
            let value = visitor.visit_newtype_struct(&mut *self)?;
            self.bytes.comma()?;

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                self.bytes.err(ErrorCode::ExpectedStructEnd)
            }
        } else {
            self.bytes.err(ErrorCode::ExpectedStruct)
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("[") {
            let value = visitor.visit_seq(CommaSeparated::new(b']', &mut self))?;
            self.bytes.comma()?;

            if self.bytes.consume("]") {
                Ok(value)
            } else {
                self.bytes.err(ErrorCode::ExpectedArrayEnd)
            }
        } else {
            self.bytes.err(ErrorCode::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("(") {
            let value = visitor.visit_seq(CommaSeparated::new(b')', &mut self))?;
            self.bytes.comma()?;

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                self.bytes.err(ErrorCode::ExpectedArrayEnd)
            }
        } else {
            self.bytes.err(ErrorCode::ExpectedArray)
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.bytes.consume(name);
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("{") {
            let value = visitor.visit_map(CommaSeparated::new(b'}', &mut self))?;
            self.bytes.comma()?;

            if self.bytes.consume("}") {
                Ok(value)
            } else {
                self.bytes.err(ErrorCode::ExpectedMapEnd)
            }
        } else {
            self.bytes.err(ErrorCode::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.bytes.consume(name);

        self.bytes.skip_ws()?;

        if self.bytes.consume("(") {
            let value = visitor.visit_map(CommaSeparated::new(b')', &mut self))?;
            self.bytes.comma()?;

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                self.bytes.err(ErrorCode::ExpectedStructEnd)
            }
        } else {
            self.bytes.err(ErrorCode::ExpectedStruct)
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(Enum::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(
            str::from_utf8(self.bytes.identifier()?).map_err(|e| self.bytes.error(e.into()))?,
        )
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    terminator: u8,
    had_comma: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(terminator: u8, de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated {
            de,
            terminator,
            had_comma: true,
        }
    }

    fn err<T>(&self, kind: ErrorCode) -> Result<T> {
        self.de.bytes.err(kind)
    }

    fn has_element(&mut self) -> Result<bool> {
        self.de.bytes.skip_ws()?;

        Ok(self.had_comma && self.de.bytes.peek_or_eof()? != self.terminator)
    }
}

impl<'de, 'a> de::SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.has_element()? {
            let res = seed.deserialize(&mut *self.de)?;

            self.had_comma = self.de.bytes.comma()?;

            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

impl<'de, 'a> de::MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.has_element()? {
            if self.terminator == b')' {
                seed.deserialize(&mut IdDeserializer::new(&mut *self.de))
                    .map(Some)
            } else {
                seed.deserialize(&mut *self.de).map(Some)
            }
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws()?;

        if self.de.bytes.consume(":") {
            self.de.bytes.skip_ws()?;

            let res = seed.deserialize(&mut TagDeserializer::new(&mut *self.de))?;

            self.had_comma = self.de.bytes.comma()?;

            Ok(res)
        } else {
            self.err(ErrorCode::ExpectedMapColon)
        }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws()?;

        let value = seed.deserialize(&mut *self.de)?;

        Ok((value, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws()?;

        if self.de.bytes.consume("(") {
            self.de.bytes.skip_ws()?;

            let val = seed.deserialize(&mut *self.de)?;

            self.de.bytes.comma()?;

            if self.de.bytes.consume(")") {
                Ok(val)
            } else {
                self.de.bytes.err(ErrorCode::ExpectedStructEnd)
            }
        } else {
            self.de.bytes.err(ErrorCode::ExpectedStruct)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.bytes.skip_ws()?;

        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.bytes.skip_ws()?;

        self.de.deserialize_struct("", fields, visitor)
    }
}
