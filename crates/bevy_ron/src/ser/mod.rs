use serde::{ser, Deserialize, Serialize};
use std::io;

use crate::{
    error::{Error, Result},
    extensions::Extensions,
};

mod value;

/// Serializes `value` into `writer`
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: Serialize,
{
    let mut s = Serializer::new(writer, None, false)?;
    value.serialize(&mut s)
}

/// Serializes `value` and returns it as string.
///
/// This function does not generate any newlines or nice formatting;
/// if you want that, you can use `to_string_pretty` instead.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let buf = Vec::new();
    let mut s = Serializer::new(buf, None, false)?;
    value.serialize(&mut s)?;
    Ok(String::from_utf8(s.output).expect("Ron should be utf-8"))
}

/// Serializes `value` in the recommended RON layout in a pretty way.
pub fn to_string_pretty<T>(value: &T, config: PrettyConfig) -> Result<String>
where
    T: Serialize,
{
    let buf = Vec::new();
    let mut s = Serializer::new(buf, Some(config), false)?;
    value.serialize(&mut s)?;
    Ok(String::from_utf8(s.output).expect("Ron should be utf-8"))
}

/// Pretty serializer state
struct Pretty {
    indent: usize,
    sequence_index: Vec<usize>,
}

/// Pretty serializer configuration.
///
/// # Examples
///
/// ```
/// use bevy_ron::ser::PrettyConfig;
///
/// let my_config = PrettyConfig::new()
///     .depth_limit(4)
///     // definitely superior (okay, just joking)
///     .indentor("\t".to_owned());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PrettyConfig {
    /// Limit the pretty-ness up to the given depth.
    #[serde(default = "default_depth_limit")]
    pub depth_limit: usize,
    /// New line string
    #[serde(default = "default_new_line")]
    pub new_line: String,
    /// Indentation string
    #[serde(default = "default_indentor")]
    pub indentor: String,
    /// Separate tuple members with indentation
    #[serde(default = "default_separate_tuple_members")]
    pub separate_tuple_members: bool,
    /// Enumerate array items in comments
    #[serde(default = "default_enumerate_arrays")]
    pub enumerate_arrays: bool,
    /// Always include the decimal in floats
    #[serde(default = "default_decimal_floats")]
    pub decimal_floats: bool,
    /// Enable extensions. Only configures 'implicit_some' for now.
    pub extensions: Extensions,
}

impl PrettyConfig {
    /// Creates a default `PrettyConfig`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Limits the pretty-formatting based on the number of indentations.
    /// I.e., with a depth limit of 5, starting with an element of depth
    /// (indentation level) 6, everything will be put into the same line,
    /// without pretty formatting.
    ///
    /// Default: [std::usize::MAX]
    pub fn depth_limit(mut self, depth_limit: usize) -> Self {
        self.depth_limit = depth_limit;

        self
    }

    /// Configures the newlines used for serialization.
    ///
    /// Default: `\r\n` on Windows, `\n` otherwise
    pub fn new_line(mut self, new_line: String) -> Self {
        self.new_line = new_line;

        self
    }

    /// Configures the string sequence used for indentation.
    ///
    /// Default: 4 spaces
    pub fn indentor(mut self, indentor: String) -> Self {
        self.indentor = indentor;

        self
    }

    /// Configures whether tuples are single- or multi-line.
    /// If set to `true`, tuples will have their fields indented and in new
    /// lines. If set to `false`, tuples will be serialized without any
    /// newlines or indentations.
    ///
    /// Default: `false`
    pub fn separate_tuple_members(mut self, separate_tuple_members: bool) -> Self {
        self.separate_tuple_members = separate_tuple_members;

        self
    }

    /// Configures whether a comment shall be added to every array element,
    /// indicating the index.
    ///
    /// Default: `false`
    pub fn enumerate_arrays(mut self, enumerate_arrays: bool) -> Self {
        self.enumerate_arrays = enumerate_arrays;

        self
    }

    /// Configures whether floats should always include a decimal.
    /// When false `1.0` will serialize as `1`
    /// When true `1.0` will serialize as `1.0`
    ///
    /// Default: `false`
    pub fn decimal_floats(mut self, decimal_floats: bool) -> Self {
        self.decimal_floats = decimal_floats;

        self
    }

    /// Configures extensions
    ///
    /// Default: Extensions::empty()
    pub fn extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;

        self
    }
}

fn default_depth_limit() -> usize {
    !0
}

fn default_new_line() -> String {
    #[cfg(not(target_os = "windows"))]
    let new_line = "\n".to_string();
    #[cfg(target_os = "windows")]
    let new_line = "\r\n".to_string();

    new_line
}

fn default_decimal_floats() -> bool {
    false
}

fn default_indentor() -> String {
    "    ".to_string()
}

fn default_separate_tuple_members() -> bool {
    false
}

fn default_enumerate_arrays() -> bool {
    false
}

impl Default for PrettyConfig {
    fn default() -> Self {
        PrettyConfig {
            depth_limit: default_depth_limit(),
            new_line: default_new_line(),
            indentor: default_indentor(),
            separate_tuple_members: default_separate_tuple_members(),
            enumerate_arrays: default_enumerate_arrays(),
            extensions: Extensions::default(),
            decimal_floats: default_decimal_floats(),
        }
    }
}

/// The RON serializer.
///
/// You can just use `to_string` for deserializing a value.
/// If you want it pretty-printed, take a look at the `pretty` module.
pub struct Serializer<W: io::Write> {
    output: W,
    pretty: Option<(PrettyConfig, Pretty)>,
    struct_names: bool,
    is_empty: Option<bool>,
}

impl<W: io::Write> Serializer<W> {
    /// Creates a new `Serializer`.
    ///
    /// Most of the time you can just use `to_string` or `to_string_pretty`.
    pub fn new(mut writer: W, config: Option<PrettyConfig>, struct_names: bool) -> Result<Self> {
        if let Some(conf) = &config {
            if conf.extensions.contains(Extensions::IMPLICIT_SOME) {
                writer.write_all(b"#![enable(implicit_some)]")?;
                writer.write_all(conf.new_line.as_bytes())?;
            };
        };
        Ok(Serializer {
            output: writer,
            pretty: config.map(|conf| {
                (
                    conf,
                    Pretty {
                        indent: 0,
                        sequence_index: Vec::new(),
                    },
                )
            }),
            struct_names,
            is_empty: None,
        })
    }

    fn is_pretty(&self) -> bool {
        match self.pretty {
            Some((ref config, ref pretty)) => pretty.indent <= config.depth_limit,
            None => false,
        }
    }

    fn separate_tuple_members(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |&(ref config, _)| config.separate_tuple_members)
    }

    fn decimal_floats(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |&(ref config, _)| config.decimal_floats)
    }

    fn extensions(&self) -> Extensions {
        self.pretty
            .as_ref()
            .map_or(Extensions::empty(), |&(ref config, _)| config.extensions)
    }

    fn start_indent(&mut self) -> Result<()> {
        if let Some((ref config, ref mut pretty)) = self.pretty {
            pretty.indent += 1;
            if pretty.indent <= config.depth_limit {
                let is_empty = self.is_empty.unwrap_or(false);

                if !is_empty {
                    self.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn indent(&mut self) -> io::Result<()> {
        if let Some((ref config, ref pretty)) = self.pretty {
            if pretty.indent <= config.depth_limit {
                for _ in 0..pretty.indent {
                    self.output.write_all(config.indentor.as_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn end_indent(&mut self) -> io::Result<()> {
        if let Some((ref config, ref mut pretty)) = self.pretty {
            if pretty.indent <= config.depth_limit {
                let is_empty = self.is_empty.unwrap_or(false);

                if !is_empty {
                    for _ in 1..pretty.indent {
                        self.output.write_all(config.indentor.as_bytes())?;
                    }
                }
            }
            pretty.indent -= 1;

            self.is_empty = None;
        }
        Ok(())
    }

    fn serialize_escaped_str(&mut self, value: &str) -> io::Result<()> {
        self.output.write_all(b"\"")?;
        let mut scalar = [0u8; 4];
        for c in value.chars().flat_map(|c| c.escape_debug()) {
            self.output
                .write_all(c.encode_utf8(&mut scalar).as_bytes())?;
        }
        self.output.write_all(b"\"")?;
        Ok(())
    }
}

impl<'a, W: io::Write> ser::Serializer for &'a mut Serializer<W> {
    type Error = Error;
    type Ok = ();
    type SerializeMap = Compound<'a, W>;
    type SerializeSeq = Compound<'a, W>;
    type SerializeStruct = Compound<'a, W>;
    type SerializeStructVariant = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeTupleStruct = Compound<'a, W>;
    type SerializeTupleVariant = Compound<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.write_all(if v { b"true" } else { b"false" })?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i128(v as i128)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i128(v as i128)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i128(v as i128)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_i128(v as i128)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        // TODO optimize
        write!(self.output, "{}", v)?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u128(v as u128)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u128(v as u128)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u128(v as u128)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_u128(v as u128)
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        write!(self.output, "{}", v)?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        write!(self.output, "{}", v)?;
        // TODO: use f32::EPSILON when minimum supported rust version is 1.43
        pub const EPSILON: f32 = 1.1920929e-7_f32;
        if self.decimal_floats() && (v - v.floor()).abs() < EPSILON {
            write!(self.output, ".0")?;
        }
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        write!(self.output, "{}", v)?;
        // TODO: use f64::EPSILON when minimum supported rust version is 1.43
        pub const EPSILON: f64 = 2.220446049250313e-16_f64;
        if self.decimal_floats() && (v - v.floor()).abs() < EPSILON {
            write!(self.output, ".0")?;
        }
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.output.write_all(b"'")?;
        if v == '\\' || v == '\'' {
            self.output.write_all(b"\\")?;
        }
        write!(self.output, "{}", v)?;
        self.output.write_all(b"'")?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_escaped_str(v)?;

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.serialize_str(base64::encode(v).as_str())
    }

    fn serialize_none(self) -> Result<()> {
        self.output.write_all(b"None")?;

        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let implicit_some = self.extensions().contains(Extensions::IMPLICIT_SOME);
        if !implicit_some {
            self.output.write_all(b"Some(")?;
        }
        value.serialize(&mut *self)?;
        if !implicit_some {
            self.output.write_all(b")")?;
        }

        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        self.output.write_all(b"()")?;

        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        if self.struct_names {
            self.output.write_all(name.as_bytes())?;

            Ok(())
        } else {
            self.serialize_unit()
        }
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<()> {
        self.output.write_all(variant.as_bytes())?;

        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.struct_names {
            self.output.write_all(name.as_bytes())?;
        }

        self.output.write_all(b"(")?;
        value.serialize(&mut *self)?;
        self.output.write_all(b")")?;
        Ok(())
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output.write_all(variant.as_bytes())?;
        self.output.write_all(b"(")?;

        value.serialize(&mut *self)?;

        self.output.write_all(b")")?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output.write_all(b"[")?;

        if let Some(len) = len {
            self.is_empty = Some(len == 0);
        }

        self.start_indent()?;

        if let Some((_, ref mut pretty)) = self.pretty {
            pretty.sequence_index.push(0);
        }

        Ok(Compound {
            ser: self,
            state: State::First,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.output.write_all(b"(")?;

        if self.separate_tuple_members() {
            self.is_empty = Some(len == 0);

            self.start_indent()?;
        }

        Ok(Compound {
            ser: self,
            state: State::First,
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        if self.struct_names {
            self.output.write_all(name.as_bytes())?;
        }

        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.output.write_all(variant.as_bytes())?;
        self.output.write_all(b"(")?;

        if self.separate_tuple_members() {
            self.is_empty = Some(len == 0);

            self.start_indent()?;
        }

        Ok(Compound {
            ser: self,
            state: State::First,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.output.write_all(b"{")?;

        if let Some(len) = len {
            self.is_empty = Some(len == 0);
        }

        self.start_indent()?;

        Ok(Compound {
            ser: self,
            state: State::First,
        })
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        if self.struct_names {
            self.output.write_all(name.as_bytes())?;
        }
        self.output.write_all(b"(")?;

        self.is_empty = Some(len == 0);
        self.start_indent()?;

        Ok(Compound {
            ser: self,
            state: State::First,
        })
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.output.write_all(variant.as_bytes())?;
        self.output.write_all(b"(")?;

        self.is_empty = Some(len == 0);
        self.start_indent()?;

        Ok(Compound {
            ser: self,
            state: State::First,
        })
    }
}

pub enum State {
    First,
    Rest,
}
pub struct Compound<'a, W: io::Write> {
    ser: &'a mut Serializer<W>,
    state: State,
}

impl<'a, W: io::Write> ser::SerializeSeq for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;
            if let Some((ref config, ref mut pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    if config.enumerate_arrays {
                        assert!(config.new_line.contains('\n'));
                        let index = pretty.sequence_index.last_mut().unwrap();
                        //TODO: when /**/ comments are supported, prepend the index
                        // to an element instead of appending it.
                        write!(self.ser.output, "// [{}]", index).unwrap();
                        *index += 1;
                    }
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.indent()?;

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref mut pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.end_indent()?;

        if let Some((_, ref mut pretty)) = self.ser.pretty {
            pretty.sequence_index.pop();
        }

        self.ser.output.write_all(b"]")?;
        Ok(())
    }
}

impl<'a, W: io::Write> ser::SerializeTuple for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser
                        .output
                        .write_all(if self.ser.separate_tuple_members() {
                            config.new_line.as_bytes()
                        } else {
                            b" "
                        })?;
                }
            }
        }

        if self.ser.separate_tuple_members() {
            self.ser.indent()?;
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if self.ser.separate_tuple_members() && pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        if self.ser.separate_tuple_members() {
            self.ser.end_indent()?;
        }

        self.ser.output.write_all(b")")?;

        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a, W: io::Write> ser::SerializeTupleStruct for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a, W: io::Write> ser::SerializeTupleVariant for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a, W: io::Write> ser::SerializeMap for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;

            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.indent()?;
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.ser.output.write_all(b":")?;

        if self.ser.is_pretty() {
            self.ser.output.write_all(b" ")?;
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.end_indent()?;
        self.ser.output.write_all(b"}")?;
        Ok(())
    }
}

impl<'a, W: io::Write> ser::SerializeStruct for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;

            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.indent()?;
        self.ser.output.write_all(key.as_bytes())?;
        self.ser.output.write_all(b":")?;

        if self.ser.is_pretty() {
            self.ser.output.write_all(b" ")?;
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.end_indent()?;
        self.ser.output.write_all(b")")?;
        Ok(())
    }
}

impl<'a, W: io::Write> ser::SerializeStructVariant for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeStruct::end(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct EmptyStruct1;

    #[derive(Serialize)]
    struct EmptyStruct2 {}

    #[derive(Serialize)]
    struct MyStruct {
        x: f32,
        y: f32,
    }

    #[derive(Serialize)]
    enum MyEnum {
        A,
        B(bool),
        C(bool, f32),
        D { a: i32, b: i32 },
    }

    #[test]
    fn test_empty_struct() {
        assert_eq!(to_string(&EmptyStruct1).unwrap(), "()");
        assert_eq!(to_string(&EmptyStruct2 {}).unwrap(), "()");
    }

    #[test]
    fn test_struct() {
        let my_struct = MyStruct { x: 4.0, y: 7.0 };

        assert_eq!(to_string(&my_struct).unwrap(), "(x:4,y:7)");

        #[derive(Serialize)]
        struct NewType(i32);

        assert_eq!(to_string(&NewType(42)).unwrap(), "(42)");

        #[derive(Serialize)]
        struct TupleStruct(f32, f32);

        assert_eq!(to_string(&TupleStruct(2.0, 5.0)).unwrap(), "(2,5)");
    }

    #[test]
    fn test_option() {
        assert_eq!(to_string(&Some(1u8)).unwrap(), "Some(1)");
        assert_eq!(to_string(&None::<u8>).unwrap(), "None");
    }

    #[test]
    fn test_enum() {
        assert_eq!(to_string(&MyEnum::A).unwrap(), "A");
        assert_eq!(to_string(&MyEnum::B(true)).unwrap(), "B(true)");
        assert_eq!(to_string(&MyEnum::C(true, 3.5)).unwrap(), "C(true,3.5)");
        assert_eq!(to_string(&MyEnum::D { a: 2, b: 3 }).unwrap(), "D(a:2,b:3)");
    }

    #[test]
    fn test_array() {
        let empty: [i32; 0] = [];
        assert_eq!(to_string(&empty).unwrap(), "()");
        let empty_ref: &[i32] = &empty;
        assert_eq!(to_string(&empty_ref).unwrap(), "[]");

        assert_eq!(to_string(&[2, 3, 4i32]).unwrap(), "(2,3,4)");
        assert_eq!(to_string(&(&[2, 3, 4i32] as &[i32])).unwrap(), "[2,3,4]");
    }

    #[test]
    fn test_map() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert((true, false), 4);
        map.insert((false, false), 123);

        let s = to_string(&map).unwrap();
        s.starts_with("{");
        s.contains("(true,false):4");
        s.contains("(false,false):123");
        s.ends_with("}");
    }

    #[test]
    fn test_string() {
        assert_eq!(to_string(&"Some string").unwrap(), "\"Some string\"");
    }

    #[test]
    fn test_char() {
        assert_eq!(to_string(&'c').unwrap(), "'c'");
    }

    #[test]
    fn test_escape() {
        assert_eq!(to_string(&r#""Quoted""#).unwrap(), r#""\"Quoted\"""#);
    }

    #[test]
    fn test_byte_stream() {
        use serde_bytes;

        let small: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        assert_eq!(
            to_string(&small).unwrap(),
            "(0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15)"
        );

        let large = vec![255u8; 64];
        let large = serde_bytes::Bytes::new(&large);
        assert_eq!(
            to_string(&large).unwrap(),
            concat!(
                "\"/////////////////////////////////////////",
                "////////////////////////////////////////////w==\""
            )
        );
    }
}
