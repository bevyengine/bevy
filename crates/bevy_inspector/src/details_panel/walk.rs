//! The reflection walk that flattens a component value into field entries.

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};

use bevy_color::{
    Color, Hsla, Hsva, Hwba, Laba, Lcha, LinearRgba, Okhsla, Okhsva, Okhwba, Oklaba, Oklcha, Srgba,
    Xyza,
};
use bevy_ecs::{entity::Entity, name::Name};
use bevy_reflect::{
    enums::{VariantInfo, VariantType},
    NamedField, PartialReflect, ReflectFromReflect, ReflectRef, TypeInfo, TypeRegistry,
    UnnamedField,
};
use bevy_ui_widgets::{NumericRange, NumericValue};
use bevy_utils::prelude::ShortName;

use super::{
    annotations::{annotations_of, narrowed_limit, FieldAnnotations},
    edit::has_defaults,
    rows::color_to_hex,
    FieldEntry, FieldValue, MAX_DEPTH, MAX_ITEMS,
};

/// Flattens a reflected component value into the rows the details panel renders.
///
/// `registry` decides which enum variants can be built from defaults and so offered for switching.
pub fn field_entries(value: &dyn PartialReflect, registry: &TypeRegistry) -> Vec<FieldEntry> {
    walk_entries(value, registry, None)
}

/// Flattens `value` into rows like [`field_entries`], skipping the rows that are not on the way
/// to the `target` path, if any.
pub(super) fn walk_entries(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    target: Option<&str>,
) -> Vec<FieldEntry> {
    let concrete = value
        .try_as_reflect()
        .is_none()
        .then(|| {
            let info = value.get_represented_type_info()?;
            registry
                .get_type_data::<ReflectFromReflect>(info.type_id())?
                .from_reflect(value)
        })
        .flatten();
    let value = concrete
        .as_deref()
        .map(PartialReflect::as_partial_reflect)
        .unwrap_or(value);
    let mut walk = Walk {
        registry,
        target,
        entries: Vec::new(),
    };
    if let Some(name) = value.try_downcast_ref::<Name>() {
        walk.push(
            String::new(),
            "value".to_string(),
            0,
            FieldValue::Label(name.as_str().to_string()),
            &FieldAnnotations::default(),
        );
        return walk.entries;
    }
    let root = FieldAnnotations::of_field_type(value.get_represented_type_info());
    let (value, path, annotations) = flatten_newtypes(value, String::new(), root);
    let label = |name: &str| annotations.display_name.unwrap_or(name).to_string();
    let editable = !annotations.read_only;
    if let Some(caption) = asset_handle_label(value) {
        walk.push(
            path,
            label("value"),
            0,
            FieldValue::Label(caption),
            &annotations,
        );
    } else if let Some(scalar) = scalar_value(value) {
        walk.push_scalar(path, label("value"), 0, value, scalar, &annotations);
    } else if let Some(variant) = variant_value(value, registry) {
        walk.push_variant(
            path.clone(),
            label("variant"),
            0,
            value,
            variant,
            &annotations,
        );
        walk.children(value, &path, 0, editable, &annotations);
    } else if let Some(summary) = summary(value) {
        if !summary.is_empty() {
            walk.push(
                path.clone(),
                label("value"),
                0,
                FieldValue::Label(summary),
                &annotations,
            );
        }
        walk.children(value, &path, 0, editable, &annotations);
    } else {
        walk.push(
            path,
            label("value"),
            0,
            FieldValue::Label(format_fallback(value)),
            &annotations,
        );
    }
    walk.entries
}

/// The state of a walk over a reflected value.
struct Walk<'a> {
    registry: &'a TypeRegistry,
    target: Option<&'a str>,
    entries: Vec<FieldEntry>,
}

impl Walk<'_> {
    fn push(
        &mut self,
        path: String,
        label: String,
        depth: usize,
        value: FieldValue,
        annotations: &FieldAnnotations,
    ) {
        self.push_entry(FieldEntry {
            path,
            label,
            depth,
            value,
            limit: None,
            annotations: annotations.clone(),
            unit_enum: false,
        });
    }

    /// Adds the row of `variant`, the variants read from the enum `value`.
    fn push_variant(
        &mut self,
        path: String,
        label: String,
        depth: usize,
        value: &dyn PartialReflect,
        variant: FieldValue,
        annotations: &FieldAnnotations,
    ) {
        self.push_entry(FieldEntry {
            path,
            label,
            depth,
            value: variant,
            limit: None,
            annotations: annotations.clone(),
            unit_enum: is_unit_enum(value),
        });
    }

    /// Adds the row of `scalar`, the value read from `value`, keeping the range of a number.
    fn push_scalar(
        &mut self,
        path: String,
        label: String,
        depth: usize,
        value: &dyn PartialReflect,
        scalar: FieldValue,
        annotations: &FieldAnnotations,
    ) {
        let limit = match scalar {
            FieldValue::Number(number) => {
                narrowed_limit(integer_limit(value), annotations.range.as_ref(), number)
            }
            _ => None,
        };
        self.push_entry(FieldEntry {
            path,
            label,
            depth,
            value: scalar,
            limit,
            annotations: annotations.clone(),
            unit_enum: false,
        });
    }

    fn push_entry(&mut self, entry: FieldEntry) {
        self.entries.push(if entry.annotations.read_only {
            entry.into_read_only()
        } else {
            entry
        });
    }

    /// Adds the rows of `value`, rendering them read-only unless `editable`.
    fn field(
        &mut self,
        value: &dyn PartialReflect,
        path: String,
        label: String,
        depth: usize,
        editable: bool,
        annotations: FieldAnnotations,
    ) {
        let on_target = self.target.is_none_or(|target| {
            target
                .strip_prefix(path.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '[']))
        });
        if !on_target {
            return;
        }
        let annotations = annotations.or(FieldAnnotations::of_field_type(
            value.get_represented_type_info(),
        ));
        let (value, path, mut annotations) = flatten_newtypes(value, path, annotations);
        annotations.read_only |= !editable;
        let editable = !annotations.read_only;
        let label = annotations
            .display_name
            .map(ToString::to_string)
            .unwrap_or(label);

        if depth >= MAX_DEPTH {
            let value = FieldValue::Label("...".to_string());
            self.push(path, label, depth, value, &annotations);
            return;
        }

        if let Some(caption) = asset_handle_label(value) {
            self.push(path, label, depth, FieldValue::Label(caption), &annotations);
            return;
        }

        if let Some(scalar) = scalar_value(value) {
            self.push_scalar(path, label, depth, value, scalar, &annotations);
            return;
        }

        if let Some(variant) = variant_value(value, self.registry) {
            self.push_variant(path.clone(), label, depth, value, variant, &annotations);
            self.children(value, &path, depth + 1, editable, &annotations);
            return;
        }

        let Some(summary) = summary(value) else {
            let value = FieldValue::Label(format_fallback(value));
            self.push(path, label, depth, value, &annotations);
            return;
        };

        let summary = FieldValue::Label(summary);
        self.push(path.clone(), label, depth, summary, &annotations);
        self.children(value, &path, depth + 1, editable, &annotations);
    }

    /// Adds the rows of the fields or items of `value`, whose own annotations are `annotations`.
    fn children(
        &mut self,
        value: &dyn PartialReflect,
        prefix: &str,
        depth: usize,
        editable: bool,
        annotations: &FieldAnnotations,
    ) {
        let constraints = annotations.value_constraints();
        match value.reflect_ref() {
            ReflectRef::Struct(value) => {
                let info = value.get_represented_struct_info();
                for index in 0..value.field_len() {
                    let Some(field) = value.field_at(index) else {
                        continue;
                    };
                    let name = value
                        .name_at(index)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string());
                    let annotations = annotations_of(
                        info.and_then(|info| info.field_at(index))
                            .map(NamedField::custom_attributes),
                    );
                    self.field(
                        field,
                        join(prefix, &name),
                        name,
                        depth,
                        editable,
                        annotations,
                    );
                }
            }
            ReflectRef::TupleStruct(value) => {
                let info = value.get_represented_tuple_struct_info();
                for (index, field) in value.iter_fields().enumerate() {
                    let name = index.to_string();
                    let annotations = annotations_of(
                        info.and_then(|info| info.field_at(index))
                            .map(UnnamedField::custom_attributes),
                    );
                    self.field(
                        field,
                        join(prefix, &name),
                        name,
                        depth,
                        editable,
                        annotations,
                    );
                }
            }
            ReflectRef::Tuple(value) => {
                for (index, field) in value.iter_fields().enumerate() {
                    let name = index.to_string();
                    let annotations = FieldAnnotations::default();
                    self.field(
                        field,
                        join(prefix, &name),
                        name,
                        depth,
                        editable,
                        annotations,
                    );
                }
            }
            ReflectRef::List(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = format!("{prefix}[{index}]");
                    let annotations = constraints.clone();
                    self.field(item, path, index.to_string(), depth, editable, annotations);
                }
            }
            ReflectRef::Array(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = format!("{prefix}[{index}]");
                    let annotations = constraints.clone();
                    self.field(item, path, index.to_string(), depth, editable, annotations);
                }
            }
            ReflectRef::Map(value) => {
                for (index, (key, item)) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = entry_path(prefix, key, index);
                    let editable = editable && key_access(key).is_some();
                    let annotations = constraints.clone();
                    self.field(item, path, display_value(key), depth, editable, annotations);
                }
            }
            ReflectRef::Set(value) => {
                for (index, item) in value.iter().take(MAX_ITEMS).enumerate() {
                    let path = entry_path(prefix, item, index);
                    let annotations = constraints.clone();
                    self.field(item, path, index.to_string(), depth, false, annotations);
                }
            }
            ReflectRef::Enum(value) => {
                let is_option = value
                    .get_represented_type_info()
                    .is_some_and(|info| info.type_path().starts_with(OPTION_PREFIX));
                let variant = value
                    .get_represented_enum_info()
                    .and_then(|info| info.variant(value.variant_name()));
                for index in 0..value.field_len() {
                    let Some(field) = value.field_at(index) else {
                        continue;
                    };
                    let name = value
                        .name_at(index)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| index.to_string());
                    let mut annotations = annotations_of(match variant {
                        Some(VariantInfo::Struct(info)) => {
                            info.field_at(index).map(NamedField::custom_attributes)
                        }
                        Some(VariantInfo::Tuple(info)) => {
                            info.field_at(index).map(UnnamedField::custom_attributes)
                        }
                        _ => None,
                    });
                    if is_option {
                        annotations = annotations.or(constraints.clone());
                    }
                    self.field(
                        field,
                        join(prefix, &name),
                        name,
                        depth,
                        editable,
                        annotations,
                    );
                }
            }
            _ => {}
        }
    }
}

/// The read-only caption of an asset handle, or `None` if `value` is not one.
///
/// Handles are matched by type path, since this crate does not depend on `bevy_asset`.
fn asset_handle_label(value: &dyn PartialReflect) -> Option<String> {
    let type_path = value.get_represented_type_info()?.type_path();
    type_path
        .starts_with(ASSET_HANDLE_PREFIX)
        .then(|| ShortName(type_path).to_string())
}

pub(super) const ASSET_HANDLE_PREFIX: &str = "bevy_asset::handle::Handle<";

/// The type path prefix of `Option`, whose `Some` payload takes the constraints of its field.
const OPTION_PREFIX: &str = "core::option::Option<";

/// The read-only caption form of a field value.
pub(crate) fn read_only(value: FieldValue) -> FieldValue {
    FieldValue::Label(match value {
        FieldValue::Bool(value) => value.to_string(),
        FieldValue::Number(NumericValue::F32(value)) => value.to_string(),
        FieldValue::Number(NumericValue::F64(value)) => value.to_string(),
        FieldValue::Number(NumericValue::I32(value)) => value.to_string(),
        FieldValue::Number(NumericValue::I64(value)) => value.to_string(),
        FieldValue::Text(text) | FieldValue::Label(text) => text,
        FieldValue::Color(color) => color_to_hex(color),
        FieldValue::Variant { variants, selected } => {
            variants.get(selected).cloned().unwrap_or_default()
        }
    })
}

/// Follows single-field tuple structs down to their inner value, returning it, its path, and
/// `annotations` filled with those of the inner fields and their types.
///
/// This keeps newtypes from adding a nesting level of their own.
fn flatten_newtypes(
    value: &dyn PartialReflect,
    mut path: String,
    mut annotations: FieldAnnotations,
) -> (&dyn PartialReflect, String, FieldAnnotations) {
    let mut value = value;
    while let ReflectRef::TupleStruct(tuple) = value.reflect_ref() {
        if tuple.field_len() != 1 {
            break;
        }
        let Some(field) = tuple.field(0) else {
            break;
        };
        let inner = tuple
            .get_represented_tuple_struct_info()
            .and_then(|info| info.field_at(0))
            .map(UnnamedField::custom_attributes);
        annotations = annotations
            .or(annotations_of(inner))
            .or(FieldAnnotations::of_field_type(
                field.get_represented_type_info(),
            ));
        path = join(&path, "0");
        value = field;
    }
    (value, path, annotations)
}

/// The caption shown on the row of a container value, or `None` if the value has no fields.
fn summary(value: &dyn PartialReflect) -> Option<String> {
    Some(match value.reflect_ref() {
        ReflectRef::Struct(_) | ReflectRef::TupleStruct(_) | ReflectRef::Tuple(_) => String::new(),
        ReflectRef::List(value) => format!("{} items", value.len()),
        ReflectRef::Array(value) => format!("{} items", value.len()),
        ReflectRef::Map(value) => format!("{} items", value.len()),
        ReflectRef::Set(value) => format!("{} items", value.len()),
        ReflectRef::Enum(value) => value.variant_name().to_string(),
        _ => return None,
    })
}

/// Formats a value that has no dedicated widget, preferring the reflected [`Display`] output.
///
/// Opaque types without a [`Display`] implementation fall back to their shortened type path.
///
/// [`Display`]: core::fmt::Display
fn format_fallback(value: &dyn PartialReflect) -> String {
    let displayed = format!("{value}");
    let displayed = if displayed.trim().is_empty() {
        format!("{value:?}")
    } else {
        displayed
    };
    let type_path = value.reflect_type_path();
    if displayed.trim() == format!("Reflect({type_path})") {
        return ShortName(type_path).to_string();
    }
    displayed
}

pub(super) fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

/// The path of the map or set entry at `index` with the given key.
///
/// Keys that [`key_access`] cannot express get a `[#index]` placeholder, which keeps the row
/// tracked but never resolves.
fn entry_path(prefix: &str, key: &dyn PartialReflect, index: usize) -> String {
    match key_access(key) {
        Some(access) => format!("{prefix}{access}"),
        None => format!("{prefix}[#{index}]"),
    }
}

/// The [`bevy_reflect::GetPath`] access selecting `key` in a map or set.
///
/// Returns `None` for keys other than strings and primitive integers.
fn key_access(key: &dyn PartialReflect) -> Option<String> {
    if let Some(key) = key.try_downcast_ref::<String>() {
        return Some(quoted_key(key));
    }
    if let Some(key) = key.try_downcast_ref::<Cow<'static, str>>() {
        return Some(quoted_key(key));
    }
    integer_key(key).map(|integer| quoted_key(&integer))
}

fn quoted_key(key: &str) -> String {
    let mut quoted = String::with_capacity(key.len() + 4);
    quoted.push_str("[\"");
    for c in key.chars() {
        if matches!(c, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push_str("\"]");
    quoted
}

fn integer_key(key: &dyn PartialReflect) -> Option<String> {
    macro_rules! integers {
        ($($type:ty),*) => {
            $(
                if let Some(key) = key.try_downcast_ref::<$type>() {
                    return Some(key.to_string());
                }
            )*
        };
    }

    integers!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    None
}

fn display_value(value: &dyn PartialReflect) -> String {
    format!("{value:?}")
}

pub(super) fn scalar_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    if let Some(value) = value.try_downcast_ref::<bool>() {
        return Some(FieldValue::Bool(*value));
    }
    if let Some(value) = value.try_downcast_ref::<f32>() {
        return Some(FieldValue::Number(NumericValue::F32(*value)));
    }
    if let Some(value) = value.try_downcast_ref::<f64>() {
        return Some(FieldValue::Number(NumericValue::F64(*value)));
    }
    if let Some(value) = integer_value(value) {
        return Some(value);
    }
    if let Some(value) = value.try_downcast_ref::<char>() {
        return Some(FieldValue::Text(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<String>() {
        return Some(FieldValue::Text(value.clone()));
    }
    if let Some(value) = value.try_downcast_ref::<Cow<'static, str>>() {
        return Some(FieldValue::Text(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<&'static str>() {
        return Some(FieldValue::Label(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<Entity>() {
        return Some(FieldValue::Label(value.to_string()));
    }
    if let Some(value) = value.try_downcast_ref::<Color>() {
        return Some(FieldValue::Color(*value));
    }
    color_value(value)
}

fn color_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    macro_rules! color {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(FieldValue::Color(Color::from(*value)));
                }
            )*
        };
    }

    color!(
        Srgba, LinearRgba, Hsla, Hsva, Hwba, Laba, Lcha, Oklaba, Oklcha, Xyza, Okhsla, Okhsva,
        Okhwba
    );
    None
}

/// Classifies an integer, falling back to a caption when it does not fit the number input.
fn integer_value(value: &dyn PartialReflect) -> Option<FieldValue> {
    macro_rules! narrow {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(FieldValue::Number(NumericValue::I32(i32::from(*value))));
                }
            )*
        };
    }
    macro_rules! wide {
        ($($type:ty),*) => {
            $(
                if let Some(value) = value.try_downcast_ref::<$type>() {
                    return Some(match i64::try_from(*value) {
                        Ok(value) => FieldValue::Number(NumericValue::I64(value)),
                        Err(_) => FieldValue::Label(value.to_string()),
                    });
                }
            )*
        };
    }

    narrow!(i8, i16, i32, u8, u16);
    wide!(i64, i128, isize, u32, u64, u128, usize);
    None
}

/// The range of an integer type that is narrower than the number input showing it.
fn integer_limit(value: &dyn PartialReflect) -> Option<NumericRange> {
    macro_rules! limit {
        ($variant:ident, $repr:ty, $($type:ty),*) => {
            $(
                if value.try_downcast_ref::<$type>().is_some() {
                    return Some(NumericRange::$variant(
                        <$repr>::try_from(<$type>::MIN).unwrap_or(<$repr>::MIN)
                            ..=<$repr>::try_from(<$type>::MAX).unwrap_or(<$repr>::MAX),
                    ));
                }
            )*
        };
    }

    limit!(I32, i32, i8, i16, u8, u16);
    limit!(I64, i64, isize, u32, u64, usize);
    None
}

/// The variants an enum can switch to: its current variant, and every variant whose fields have
/// registered defaults.
///
/// Returns `None` for an enum with data that cannot switch to any other variant.
fn variant_value(value: &dyn PartialReflect, registry: &TypeRegistry) -> Option<FieldValue> {
    let ReflectRef::Enum(reflected) = value.reflect_ref() else {
        return None;
    };
    let TypeInfo::Enum(info) = value.get_represented_type_info()? else {
        return None;
    };
    let current = reflected.variant_name();
    let variants: Vec<String> = info
        .iter()
        .filter(|variant| variant.name() == current || has_defaults(variant, registry))
        .map(|variant| variant.name().to_string())
        .collect();
    if !is_unit_enum(value) && variants.len() < 2 {
        return None;
    }
    let selected = variants.iter().position(|name| name == current)?;
    Some(FieldValue::Variant { variants, selected })
}

/// Whether `value` is an enum whose variants are all unit variants.
fn is_unit_enum(value: &dyn PartialReflect) -> bool {
    match value.get_represented_type_info() {
        Some(TypeInfo::Enum(info)) => info
            .iter()
            .all(|variant| variant.variant_type() == VariantType::Unit),
        _ => false,
    }
}
