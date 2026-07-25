//! Code for displaying reflected values in a consistent, human-readable format.
//!
//! Primarily useful for interactive workflows,
//! such as inspecting reflected values in a text-based format,
//! or logging them to the console.
//! Because the output is pure text, it is particularly useful for automated inspection,
//! println!-debugging, and as output for screen readers.
//!
//! Be mindful: these [`Display`] implementations return the full type path for each contained type.
//! While useful for detailed inspection, it can be unhelpfully verbose, especially for nested values.
//! To recursively collapse all contained type paths to their "short names" (i.e. with no crate/module paths),
//! use [`disqualified::ShortName::from`] on the returned [`String`](alloc::string::String),
//! then use the [`Display`] implementation on [`ShortName`](disqualified::ShortName) for more concise output.
//!
//! These implementations are stored in their own module rather than beside the types themselves
//! to help ensure consistency and reduce clutter.
//!
//! Note that these implementations are for the trait objects (`dyn Struct`, etc.) rather than for every `T: Struct`.
//! This is deliberate: it reduces the compile time and binary costs
//! associated with generating this code for every type that implements `Reflect`.
//! To use them with a concrete type, cast your value to a trait object before formatting it:
//! e.g. `format!("{}", &my_value as &dyn Reflect)`.

use alloc::vec::Vec;
use core::any::TypeId;
use core::fmt::{Display, Formatter, Write};

#[cfg(feature = "functions")]
use crate::func::Function;
use crate::{
    array::Array,
    enums::{Enum, VariantType},
    list::List,
    map::Map,
    set::Set,
    structs::Struct,
    tuple::Tuple,
    tuple_struct::TupleStruct,
    PartialReflect, Reflect, ReflectRef, TypeInfo,
};

/// String-formats a reflected value, detecting cycles via their [`TypeId`].
///
/// If `value`'s [`TypeId`] already appears in `ancestry`, the output is
/// truncated to `{type_path} { ... }` to avoid infinite recursion.
///
/// This catches both direct recursion (a type containing itself)
/// and mutual recursion (type A containing type B containing type A).
///
/// Dynamic types without type info cannot be cycle-checked.
///
/// `indent` is the indentation level at which this value's closing delimiter
/// (or last continuation line for opaque values) should appear.
fn write_value(
    f: &mut Formatter<'_>,
    value: &dyn PartialReflect,
    // A HashSet / BTreeSet has better asymptotic performance for the .contains() check,
    // but a Vec will be faster at low n, which is the common case for reflected values,
    // as most types are not deeply nested.
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    // Dynamic types without type info cannot be cycle-checked.
    let Some(type_info) = value.get_represented_type_info() else {
        return write_reflect_ref(f, &value.reflect_ref(), ancestry, indent);
    };

    let type_id = type_info.type_id();
    let type_path = type_info.type_path();

    if ancestry.contains(&type_id) {
        return write!(f, "{type_path} {{ ... }}");
    }

    ancestry.push(type_id);
    let result = write_reflect_ref(f, &value.reflect_ref(), ancestry, indent);
    ancestry.pop();
    result
}

/// Writes a [`ReflectRef`], dispatching to the type-specific writer.
fn write_reflect_ref(
    f: &mut Formatter<'_>,
    reflect_ref: &ReflectRef<'_>,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    match *reflect_ref {
        ReflectRef::Struct(value) => write_struct(f, value, ancestry, indent),
        ReflectRef::TupleStruct(value) => write_tuple_struct(f, value, ancestry, indent),
        ReflectRef::Tuple(value) => write_tuple(f, value, ancestry, indent),
        ReflectRef::List(value) => write_list(f, value, ancestry, indent),
        ReflectRef::Array(value) => write_array(f, value, ancestry, indent),
        ReflectRef::Map(value) => write_map(f, value, ancestry, indent),
        ReflectRef::Set(value) => write_set(f, value, ancestry, indent),
        ReflectRef::Enum(value) => write_enum(f, value, ancestry, indent),
        ReflectRef::Opaque(value) => write_opaque(f, value, indent),
        #[cfg(feature = "functions")]
        ReflectRef::Function(function) => write_function(f, function),
    }
}

/// Writes a reflected [`Struct`] value.
fn write_struct(
    f: &mut Formatter<'_>,
    value: &dyn Struct,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    let type_name = display_type_name(value.get_represented_type_info(), "<Unknown Struct>");

    if value.field_len() == 0 {
        return write!(f, "{type_name} {{}}");
    }

    write_delimited_block(f, type_name, " ", '{', '}', indent, |fmt, entry_indent| {
        for i in 0..value.field_len() {
            let field_name = value.name_at(i).unwrap_or("<Unknown Field>");
            write_indent(fmt, entry_indent)?;
            fmt.write_str(field_name)?;
            fmt.write_str(": ")?;
            match value.field_at(i) {
                Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                None => fmt.write_str("<None>")?,
            }
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`TupleStruct`] value.
fn write_tuple_struct(
    f: &mut Formatter<'_>,
    value: &dyn TupleStruct,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    let type_name = display_type_name(value.get_represented_type_info(), "<Unknown TupleStruct>");

    if value.field_len() == 0 {
        return write!(f, "{type_name}()");
    }

    write_delimited_block(f, type_name, "", '(', ')', indent, |fmt, entry_indent| {
        for i in 0..value.field_len() {
            write_indent(fmt, entry_indent)?;
            match value.field(i) {
                Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                None => fmt.write_str("<None>")?,
            }
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`Tuple`] value.
fn write_tuple(
    f: &mut Formatter<'_>,
    value: &dyn Tuple,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    if value.field_len() == 0 {
        return f.write_str("()");
    }

    write_delimited_block(f, "", "", '(', ')', indent, |fmt, entry_indent| {
        for i in 0..value.field_len() {
            write_indent(fmt, entry_indent)?;
            match value.field(i) {
                Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                None => fmt.write_str("<None>")?,
            }
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`List`] value.
fn write_list(
    f: &mut Formatter<'_>,
    value: &dyn List,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    if value.is_empty() {
        return f.write_str("[]");
    }

    write_delimited_block(f, "", "", '[', ']', indent, |fmt, entry_indent| {
        for i in 0..value.len() {
            write_indent(fmt, entry_indent)?;
            match value.get(i) {
                Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                None => fmt.write_str("<None>")?,
            }
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`Array`] value.
fn write_array(
    f: &mut Formatter<'_>,
    value: &dyn Array,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    if value.is_empty() {
        return f.write_str("[]");
    }

    write_delimited_block(f, "", "", '[', ']', indent, |fmt, entry_indent| {
        for i in 0..value.len() {
            write_indent(fmt, entry_indent)?;
            match value.get(i) {
                Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                None => fmt.write_str("<None>")?,
            }
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`Map`] value.
fn write_map(
    f: &mut Formatter<'_>,
    value: &dyn Map,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    if value.is_empty() {
        return f.write_str("{}");
    }

    write_delimited_block(f, "", "", '{', '}', indent, |fmt, entry_indent| {
        for (key, key_value) in value.iter() {
            write_indent(fmt, entry_indent)?;
            write_value(fmt, key, ancestry, entry_indent)?;
            fmt.write_str(": ")?;
            write_value(fmt, key_value, ancestry, entry_indent)?;
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`Set`] value.
fn write_set(
    f: &mut Formatter<'_>,
    value: &dyn Set,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    if value.is_empty() {
        return f.write_str("{}");
    }

    write_delimited_block(f, "", "", '{', '}', indent, |fmt, entry_indent| {
        for element in value.iter() {
            write_indent(fmt, entry_indent)?;
            write_value(fmt, element, ancestry, entry_indent)?;
            fmt.write_str(",\n")?;
        }
        Ok(())
    })
}

/// Writes a reflected [`Enum`] value.
fn write_enum(
    f: &mut Formatter<'_>,
    value: &dyn Enum,
    ancestry: &mut Vec<TypeId>,
    indent: u32,
) -> core::fmt::Result {
    let type_name = display_type_name(value.get_represented_type_info(), "<Unknown Enum>");
    let variant = value.variant_name();
    let variant = if variant.is_empty() {
        "<unnamed>"
    } else {
        variant
    };
    let qualified = alloc::format!("{type_name}::{variant}");

    match value.variant_type() {
        VariantType::Struct => {
            if value.field_len() == 0 {
                return write!(f, "{qualified} {{}}");
            }

            write_delimited_block(f, &qualified, " ", '{', '}', indent, |fmt, entry_indent| {
                for i in 0..value.field_len() {
                    let field_name = value.name_at(i).unwrap_or("<Unknown Field>");
                    write_indent(fmt, entry_indent)?;
                    fmt.write_str(field_name)?;
                    fmt.write_str(": ")?;
                    match value.field_at(i) {
                        Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                        None => fmt.write_str("<None>")?,
                    }
                    fmt.write_str(",\n")?;
                }
                Ok(())
            })
        }
        VariantType::Tuple => {
            if value.field_len() == 0 {
                return write!(f, "{qualified}()");
            }

            write_delimited_block(f, &qualified, "", '(', ')', indent, |fmt, entry_indent| {
                for i in 0..value.field_len() {
                    write_indent(fmt, entry_indent)?;
                    match value.field_at(i) {
                        Some(child) => write_value(fmt, child, ancestry, entry_indent)?,
                        None => fmt.write_str("<None>")?,
                    }
                    fmt.write_str(",\n")?;
                }
                Ok(())
            })
        }
        VariantType::Unit => write!(f, "{qualified}"),
    }
}

/// Writes a reflected [`Function`] value.
///
/// This method does not need cycle tracking as functions cannot contain other reflected values.
#[cfg(feature = "functions")]
fn write_function(f: &mut Formatter<'_>, func: &dyn Function) -> core::fmt::Result {
    let pretty = func
        .info()
        .pretty_printer()
        .include_fn_token()
        .include_name();
    // TODO: PrettyPrintFunctionInfo implements Debug but not Display
    // so we just use the Debug formatting for now.
    write!(f, "{pretty:?}")
}

impl Display for dyn Struct {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_struct(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn TupleStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_tuple_struct(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn Tuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_tuple(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn List {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_list(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_array(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_map(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn Set {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_set(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn Enum {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_enum(f, self, &mut Vec::new(), 0)
    }
}

#[cfg(feature = "functions")]
impl Display for dyn Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_function(f, self)
    }
}

impl Display for ReflectRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut ancestry = Vec::new();
        write_reflect_ref(f, self, &mut ancestry, 0)
    }
}

impl Display for dyn PartialReflect {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write_value(f, self, &mut Vec::new(), 0)
    }
}

impl Display for dyn Reflect {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self as &dyn PartialReflect, f)
    }
}

/// Display name for a reflected type, using its full type path.
///
/// Uses `fallback` when no type information is available (e.g. for dynamic values).
fn display_type_name(type_info: Option<&TypeInfo>, fallback: &'static str) -> &'static str {
    match type_info {
        Some(info) => info.type_path(),
        None => fallback,
    }
}

/// Writes a two-space indent for each level of indentation requested to the formatter.
fn write_indent(f: &mut Formatter<'_>, level: u32) -> core::fmt::Result {
    for _ in 0..level {
        f.write_str("  ")?;
    }
    Ok(())
}

/// Writes a delimited, indented block to `f`.
///
/// The `prefix{separator}{open}` is written first, then the closure writes
/// entries (which should use `entry_indent` for indentation), and finally
/// the closing `close` character is written at `indent`.
///
/// The caller must handle the empty case itself; this helper always emits
/// a multi-line block.
fn write_delimited_block(
    f: &mut Formatter<'_>,
    prefix: &str,
    separator: &str,
    open: char,
    close: char,
    indent: u32,
    write_entries: impl FnOnce(&mut Formatter<'_>, u32) -> core::fmt::Result,
) -> core::fmt::Result {
    let entry_indent = indent + 1;
    writeln!(f, "{prefix}{separator}{open}")?;
    write_entries(f, entry_indent)?;
    write_indent(f, indent)?;
    f.write_char(close)
}

/// Writes an opaque reflected value by delegating to its [`Debug`] implementation.
///
/// Single-line debug output is written directly. Multi-line debug output has
/// continuation lines indented one level deeper than `indent` (i.e. at the
/// same visual level as entries inside a container whose closing delimiter
/// sits at `indent`).
///
/// Leading whitespace is trimmed.
fn write_opaque(
    f: &mut Formatter<'_>,
    value: &dyn PartialReflect,
    indent: u32,
) -> core::fmt::Result {
    let debug = alloc::format!("{value:?}");
    let trimmed = debug.trim();

    // Fast path for single-line debug output
    if trimmed.len() == debug.len() && !trimmed.contains('\n') {
        return f.write_str(&debug);
    }

    // Multi-line: indent continuation lines to `indent + 1`
    // (one level deeper than the enclosing container closing delimiter).
    for (i, line) in trimmed.lines().enumerate() {
        if i > 0 {
            f.write_char('\n')?;
        }
        if line.trim().is_empty() {
            continue;
        }
        if i > 0 {
            write_indent(f, indent + 1)?;
        }
        // The first line (i == 0) is inline (no extra indent).
        f.write_str(line)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        array::DynamicArray, map::DynamicMap, set::DynamicSet, structs::DynamicStruct,
        tuple::DynamicTuple, tuple_struct::DynamicTupleStruct, Reflect,
    };
    use alloc::{
        boxed::Box,
        collections::BTreeMap,
        format,
        string::{String, ToString},
        vec,
    };
    use bevy_platform::collections::HashSet;
    use disqualified::ShortName;

    #[derive(Reflect, PartialEq, Eq, Hash)]
    #[reflect(Hash)]
    struct Inner {
        a: u32,
        b: String,
    }

    #[derive(Reflect)]
    struct Outer {
        name: String,
        inner: Inner,
        list: Vec<i32>,
    }

    #[derive(Reflect)]
    struct GraphNode {
        value: u32,
        children: Vec<GraphNode>,
    }

    #[derive(Reflect)]
    struct EmptyStruct {}

    #[derive(Reflect)]
    struct MutuallyRecursiveA {
        items: Vec<MutuallyRecursiveB>,
    }

    #[derive(Reflect)]
    struct MutuallyRecursiveB {
        items: Vec<MutuallyRecursiveA>,
    }

    #[derive(Reflect)]
    struct Newtype(u32);

    #[derive(Reflect)]
    struct EmptyTupleStruct();

    #[derive(Reflect)]
    enum MyEnum {
        Unit,
        Tuple(u32, String),
        Struct { x: i32, inner: Inner },
        EmptyTuple(),
        EmptyStruct {},
    }

    #[test]
    fn opaque_values_use_debug() {
        assert_eq!(format!("{}", &42u32 as &dyn PartialReflect), "42");
        assert_eq!(
            format!("{}", &"hi".to_string() as &dyn PartialReflect),
            "\"hi\""
        );
        assert_eq!(format!("{}", &true as &dyn PartialReflect), "true");
    }

    #[test]
    fn flat_struct() {
        let value = Inner {
            a: 1,
            b: "two".to_string(),
        };
        assert_eq!(
            format!("{}", &value as &dyn PartialReflect),
            "bevy_reflect::display::tests::Inner {\n  a: 1,\n  b: \"two\",\n}"
        );
    }

    #[test]
    fn empty_containers_collapse_to_one_line() {
        assert_eq!(
            format!("{}", &EmptyStruct {} as &dyn PartialReflect),
            "bevy_reflect::display::tests::EmptyStruct {}"
        );
        assert_eq!(
            format!("{}", &EmptyTupleStruct() as &dyn PartialReflect),
            "bevy_reflect::display::tests::EmptyTupleStruct()"
        );
        assert_eq!(
            format!("{}", &Vec::<i32>::new() as &dyn PartialReflect),
            "[]"
        );
        assert_eq!(format!("{}", &[0i32; 0] as &dyn PartialReflect), "[]");
        assert_eq!(format!("{}", &() as &dyn PartialReflect), "()");
        assert_eq!(
            format!("{}", &BTreeMap::<u32, u32>::new() as &dyn PartialReflect),
            "{}"
        );
        assert_eq!(
            format!("{}", &HashSet::<u32>::new() as &dyn PartialReflect),
            "{}"
        );
    }

    #[test]
    fn empty_enum_variants() {
        assert_eq!(
            format!("{}", &MyEnum::EmptyTuple() as &dyn PartialReflect),
            "bevy_reflect::display::tests::MyEnum::EmptyTuple()"
        );
        assert_eq!(
            format!("{}", &MyEnum::EmptyStruct {} as &dyn PartialReflect),
            "bevy_reflect::display::tests::MyEnum::EmptyStruct {}"
        );
    }

    #[test]
    fn newtype_struct() {
        assert_eq!(
            format!("{}", &Newtype(5) as &dyn PartialReflect),
            "bevy_reflect::display::tests::Newtype(\n  5,\n)"
        );
    }

    #[test]
    fn list_of_scalars() {
        assert_eq!(
            format!("{}", &vec![1, 2, 3] as &dyn PartialReflect),
            "[\n  1,\n  2,\n  3,\n]"
        );
    }

    #[test]
    fn map_entries() {
        let mut map = BTreeMap::new();
        map.insert(1u32, "one".to_string());
        map.insert(2u32, "two".to_string());
        assert_eq!(
            format!("{}", &map as &dyn PartialReflect),
            "{\n  1: \"one\",\n  2: \"two\",\n}"
        );
    }

    #[test]
    fn enum_variants() {
        assert_eq!(
            format!("{}", &MyEnum::Unit as &dyn PartialReflect),
            "bevy_reflect::display::tests::MyEnum::Unit"
        );
        assert_eq!(
            format!(
                "{}",
                &MyEnum::Tuple(7, "t".to_string()) as &dyn PartialReflect
            ),
            "bevy_reflect::display::tests::MyEnum::Tuple(\n  7,\n  \"t\",\n)"
        );
    }

    #[test]
    fn nested_values_are_indented_per_level() {
        let value = Outer {
            name: "hello".to_string(),
            inner: Inner {
                a: 1,
                b: "two".to_string(),
            },
            list: vec![10, 20],
        };

        let expected = "\
bevy_reflect::display::tests::Outer {
  name: \"hello\",
  inner: bevy_reflect::display::tests::Inner {
    a: 1,
    b: \"two\",
  },
  list: [
    10,
    20,
  ],
}";
        assert_eq!(format!("{}", &value as &dyn PartialReflect), expected);
    }

    #[test]
    fn deeply_nested_enum_struct_variant() {
        let value = MyEnum::Struct {
            x: -1,
            inner: Inner {
                a: 2,
                b: "q".to_string(),
            },
        };

        let expected = "\
bevy_reflect::display::tests::MyEnum::Struct {
  x: -1,
  inner: bevy_reflect::display::tests::Inner {
    a: 2,
    b: \"q\",
  },
}";
        assert_eq!(format!("{}", &value as &dyn PartialReflect), expected);
    }

    #[test]
    fn type_name_uses_full_path() {
        let value = Inner {
            a: 1,
            b: "two".to_string(),
        };
        assert_eq!(
            format!("{}", &value as &dyn PartialReflect),
            "bevy_reflect::display::tests::Inner {\n  a: 1,\n  b: \"two\",\n}"
        );
    }

    #[test]
    fn shortname_collapses_type_paths() {
        let value = Outer {
            name: "hello".to_string(),
            inner: Inner {
                a: 1,
                b: "two".to_string(),
            },
            list: vec![10, 20],
        };

        let full = format!("{}", &value as &dyn PartialReflect);
        // Verify the full version actually has paths to collapse
        assert!(full.contains("bevy_reflect::display::tests::Outer"));
        assert!(full.contains("bevy_reflect::display::tests::Inner"));

        let short = ShortName::from(full.as_str()).to_string();
        assert_eq!(
            short,
            concat!(
                "Outer {\n",
                "  name: \"hello\",\n",
                "  inner: Inner {\n",
                "    a: 1,\n",
                "    b: \"two\",\n",
                "  },\n",
                "  list: [\n",
                "    10,\n",
                "    20,\n",
                "  ],\n",
                "}"
            )
        );
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque, Debug)]
    struct TrailingNewlineDebug;

    impl core::fmt::Debug for TrailingNewlineDebug {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            writeln!(f, "value")
        }
    }

    #[derive(Reflect)]
    struct HoldsTrailingNewline {
        op: TrailingNewlineDebug,
    }

    #[test]
    fn opaque_debug_trailing_newline_does_not_orphan_comma() {
        assert_eq!(
            format!("{}", &TrailingNewlineDebug as &dyn PartialReflect),
            "value"
        );
        assert_eq!(
            format!(
                "{}",
                &HoldsTrailingNewline {
                    op: TrailingNewlineDebug
                } as &dyn PartialReflect
            ),
            "bevy_reflect::display::tests::HoldsTrailingNewline {\n  op: value,\n}"
        );
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque, Debug)]
    struct TrailingNewlineThenSpaces;

    impl core::fmt::Debug for TrailingNewlineThenSpaces {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "value\n  ")
        }
    }

    #[derive(Reflect)]
    struct HoldsTrailingNewlineThenSpaces {
        op: TrailingNewlineThenSpaces,
    }

    #[test]
    fn opaque_debug_trailing_whitespace_does_not_orphan_comma() {
        assert_eq!(
            format!("{}", &TrailingNewlineThenSpaces as &dyn PartialReflect),
            "value"
        );
        assert_eq!(
            format!(
                "{}",
                &HoldsTrailingNewlineThenSpaces {
                    op: TrailingNewlineThenSpaces
                } as &dyn PartialReflect
            ),
            "bevy_reflect::display::tests::HoldsTrailingNewlineThenSpaces {\n  op: value,\n}"
        );
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque, Debug)]
    struct MultiLineDebug;

    impl core::fmt::Debug for MultiLineDebug {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "line1\nline2")
        }
    }

    #[derive(Reflect)]
    struct HoldsMultiLine {
        op: MultiLineDebug,
    }

    #[test]
    fn multi_line_opaque_value_indents_continuation_lines() {
        assert_eq!(
            format!("{}", &MultiLineDebug as &dyn PartialReflect),
            "line1\n  line2"
        );
        assert_eq!(
            format!(
                "{}",
                &HoldsMultiLine { op: MultiLineDebug } as &dyn PartialReflect
            ),
            "bevy_reflect::display::tests::HoldsMultiLine {\n  op: line1\n    line2,\n}"
        );
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque, Debug)]
    struct BlankInteriorLineDebug;

    impl core::fmt::Debug for BlankInteriorLineDebug {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "line1\n   \nline2")
        }
    }

    #[derive(Reflect)]
    struct HoldsBlankInterior {
        op: BlankInteriorLineDebug,
    }

    #[test]
    fn opaque_debug_blank_interior_line_carries_no_trailing_whitespace() {
        assert_eq!(
            format!("{}", &BlankInteriorLineDebug as &dyn PartialReflect),
            "line1\n\n  line2"
        );
        assert_eq!(
            format!(
                "{}",
                &HoldsBlankInterior {
                    op: BlankInteriorLineDebug
                } as &dyn PartialReflect
            ),
            "bevy_reflect::display::tests::HoldsBlankInterior {\n  op: line1\n\n    line2,\n}"
        );
        let formatted = format!(
            "{}",
            &HoldsBlankInterior {
                op: BlankInteriorLineDebug
            } as &dyn PartialReflect
        );
        for line in formatted.lines() {
            assert_eq!(
                line.trim_end(),
                line,
                "line has trailing whitespace: {line:?}"
            );
        }
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque, Debug)]
    struct LeadingNewlineDebug;

    impl core::fmt::Debug for LeadingNewlineDebug {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "\nvalue")
        }
    }

    #[derive(Reflect)]
    struct HoldsLeadingNewline {
        op: LeadingNewlineDebug,
    }

    #[test]
    fn opaque_debug_leading_newline_does_not_leave_trailing_whitespace() {
        assert_eq!(
            format!("{}", &LeadingNewlineDebug as &dyn PartialReflect),
            "value"
        );
        assert_eq!(
            format!(
                "{}",
                &HoldsLeadingNewline {
                    op: LeadingNewlineDebug
                } as &dyn PartialReflect
            ),
            "bevy_reflect::display::tests::HoldsLeadingNewline {\n  op: value,\n}"
        );
    }

    #[test]
    fn multi_line_struct_as_list_element() {
        let value = vec![
            Inner {
                a: 1,
                b: "x".to_string(),
            },
            Inner {
                a: 2,
                b: "y".to_string(),
            },
        ];
        let expected = "\
[
  bevy_reflect::display::tests::Inner {
    a: 1,
    b: \"x\",
  },
  bevy_reflect::display::tests::Inner {
    a: 2,
    b: \"y\",
  },
]";
        assert_eq!(format!("{}", &value as &dyn PartialReflect), expected);
    }

    #[test]
    fn multi_line_struct_as_map_value() {
        let mut map = BTreeMap::new();
        map.insert(
            1u32,
            Inner {
                a: 10,
                b: "x".to_string(),
            },
        );
        let expected = "\
{
  1: bevy_reflect::display::tests::Inner {
    a: 10,
    b: \"x\",
  },
}";
        assert_eq!(format!("{}", &map as &dyn PartialReflect), expected);
    }

    #[test]
    fn map_with_struct_key() {
        let mut map = DynamicMap::default();
        map.insert(
            Inner {
                a: 1,
                b: "key".to_string(),
            },
            "value".to_string(),
        );
        assert_eq!(
            format!("{}", &map as &dyn PartialReflect),
            "{\n  bevy_reflect::display::tests::Inner {\n    a: 1,\n    b: \"key\",\n  }: \"value\",\n}"
        );
    }

    #[test]
    fn array_of_scalars() {
        let array = DynamicArray::new(
            vec![
                Box::new(1u32) as Box<dyn PartialReflect>,
                Box::new(2u32),
                Box::new(3u32),
            ]
            .into_boxed_slice(),
        );
        assert_eq!(
            format!("{}", &array as &dyn PartialReflect),
            "[\n  1,\n  2,\n  3,\n]"
        );
    }

    #[test]
    fn set_with_one_element() {
        let mut set = DynamicSet::default();
        set.insert(42u32);
        assert_eq!(format!("{}", &set as &dyn PartialReflect), "{\n  42,\n}");
    }

    #[test]
    fn set_with_struct_element() {
        let mut set = DynamicSet::default();
        set.insert(Inner {
            a: 10,
            b: "x".to_string(),
        });
        assert_eq!(
            format!("{}", &set as &dyn PartialReflect),
            "{\n  bevy_reflect::display::tests::Inner {\n    a: 10,\n    b: \"x\",\n  },\n}"
        );
    }

    #[test]
    fn dynamic_struct_fallback_name() {
        let mut dyn_struct = DynamicStruct::default();
        dyn_struct.insert("x", 1u32);
        dyn_struct.insert("y", 2u32);
        assert_eq!(
            format!("{}", &dyn_struct as &dyn PartialReflect),
            "<Unknown Struct> {\n  x: 1,\n  y: 2,\n}"
        );
    }

    #[test]
    fn dynamic_tuple_displays() {
        let mut dyn_tuple = DynamicTuple::default();
        dyn_tuple.insert(42u32);
        dyn_tuple.insert("hi".to_string());
        assert_eq!(
            format!("{}", &dyn_tuple as &dyn PartialReflect),
            "(\n  42,\n  \"hi\",\n)"
        );
    }

    #[test]
    fn dynamic_tuple_struct_fallback_name() {
        let mut dyn_tuple_struct = DynamicTupleStruct::default();
        dyn_tuple_struct.insert(42u32);
        dyn_tuple_struct.insert("hi".to_string());
        assert_eq!(
            format!("{}", &dyn_tuple_struct as &dyn PartialReflect),
            "<Unknown TupleStruct>(\n  42,\n  \"hi\",\n)"
        );
    }

    #[test]
    fn reflect_ref_displays() {
        let value = Inner {
            a: 1,
            b: "two".to_string(),
        };
        let reflect_ref = value.reflect_ref();
        assert_eq!(
            format!("{reflect_ref}"),
            "bevy_reflect::display::tests::Inner {\n  a: 1,\n  b: \"two\",\n}"
        );
    }

    #[test]
    fn cycle_detection_truncates_on_repeated_type_path() {
        let child = GraphNode {
            value: 2,
            children: Vec::new(),
        };
        let parent = GraphNode {
            value: 1,
            children: vec![child],
        };
        assert_eq!(
            format!("{}", &parent as &dyn PartialReflect),
            concat!(
                "bevy_reflect::display::tests::GraphNode {\n",
                "  value: 1,\n",
                "  children: [\n",
                "    bevy_reflect::display::tests::GraphNode { ... },\n",
                "  ],\n",
                "}"
            )
        );
    }

    #[test]
    fn cross_type_cycle_detection_truncates() {
        let inner = MutuallyRecursiveA { items: Vec::new() };
        let b = MutuallyRecursiveB { items: vec![inner] };
        let outer = MutuallyRecursiveA { items: vec![b] };
        assert_eq!(
            format!("{}", &outer as &dyn PartialReflect),
            concat!(
                "bevy_reflect::display::tests::MutuallyRecursiveA {\n",
                "  items: [\n",
                "    bevy_reflect::display::tests::MutuallyRecursiveB {\n",
                "      items: [\n",
                "        bevy_reflect::display::tests::MutuallyRecursiveA { ... },\n",
                "      ],\n",
                "    },\n",
                "  ],\n",
                "}"
            )
        );
    }

    #[cfg(feature = "functions")]
    mod functions {
        use super::*;
        use crate::func::IntoFunction;

        #[test]
        fn named_function() {
            fn greet(name: &String) -> String {
                format!("Hello, {name}!")
            }
            let function = greet.into_function();
            assert_eq!(
                format!("{}", &function as &dyn PartialReflect),
                "fn bevy_reflect::display::tests::functions::named_function::greet(_: &alloc::string::String) -> alloc::string::String"
            );
        }

        #[test]
        fn anonymous_function() {
            let function = (|a: i32, b: i32| a + b).into_function();
            assert_eq!(
                format!("{}", &function as &dyn PartialReflect),
                "fn _(_: i32, _: i32) -> i32"
            );
        }

        #[test]
        fn overloaded_function() {
            fn add_i32(a: i32, b: i32) -> i32 {
                a + b
            }
            fn add_f32(a: f32, b: f32) -> f32 {
                a + b
            }
            let function = add_i32
                .into_function()
                .with_overload(add_f32)
                .with_name("add");
            assert_eq!(
                format!("{}", &function as &dyn PartialReflect),
                "fn add {(_: i32, _: i32) -> i32, (_: f32, _: f32) -> f32}"
            );
        }
    }
}
