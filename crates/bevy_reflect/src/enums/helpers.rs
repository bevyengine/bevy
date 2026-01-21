use crate::{
    enums::{Enum, VariantType},
    utility::reflect_hasher,
    PartialReflect, ReflectRef,
};
use core::{
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// Returns the `u64` hash of the given [enum](Enum).
#[inline(never)]
pub fn enum_hash(value: &dyn Enum) -> Option<u64> {
    let mut hasher = reflect_hasher();
    core::any::Any::type_id(value).hash(&mut hasher);
    value.variant_name().hash(&mut hasher);
    value.variant_type().hash(&mut hasher);
    for field in value.iter_fields() {
        hasher.write_u64(field.value().reflect_hash()?);
    }
    Some(hasher.finish())
}

/// Compares an [`Enum`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is an enum;
/// - `b` is the same variant as `a`;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values.
#[inline(never)]
pub fn enum_partial_eq(a: &dyn Enum, b: &dyn PartialReflect) -> Option<bool> {
    // Both enums?
    let ReflectRef::Enum(b) = b.reflect_ref() else {
        return Some(false);
    };

    // Same variant name?
    if a.variant_name() != b.variant_name() {
        return Some(false);
    }

    // Same variant type?
    if !a.is_variant(b.variant_type()) {
        return Some(false);
    }

    match a.variant_type() {
        VariantType::Struct => {
            if a.field_len() != b.field_len() {
                return Some(false);
            }
            // Same struct fields?
            for field in a.iter_fields() {
                let field_name = field.name().unwrap();
                if let Some(field_value) = b.field(field_name) {
                    if let Some(false) | None = field_value.reflect_partial_eq(field.value()) {
                        // Fields failed comparison
                        return Some(false);
                    }
                } else {
                    // Field does not exist
                    return Some(false);
                }
            }
            Some(true)
        }
        VariantType::Tuple => {
            if a.field_len() != b.field_len() {
                return Some(false);
            }
            // Same tuple fields?
            for (i, field) in a.iter_fields().enumerate() {
                if let Some(field_value) = b.field_at(i) {
                    if let Some(false) | None = field_value.reflect_partial_eq(field.value()) {
                        // Fields failed comparison
                        return Some(false);
                    }
                } else {
                    // Field does not exist
                    return Some(false);
                }
            }
            Some(true)
        }
        _ => Some(true),
    }
}

/// Compares two [`Enum`] values (by variant) and returns their ordering.
///
/// Returns [`None`] if the comparison couldn't be performed (e.g., kinds mismatch
/// or an element comparison returns `None`).
///
/// The ordering is same with `derive` macro. First order by variant index, then by fields.
#[inline(never)]
pub fn enum_partial_cmp(a: &dyn Enum, b: &dyn PartialReflect) -> Option<::core::cmp::Ordering> {
    // Both enums?
    let ReflectRef::Enum(b) = b.reflect_ref() else {
        return None;
    };

    // Same variant name?
    if a.variant_name() != b.variant_name() {
        // Different variant names, determining ordering by variant index
        return Some(a.variant_index().cmp(&b.variant_index()));
    }

    // Same variant type?
    if !a.is_variant(b.variant_type()) {
        return None;
    }

    match a.variant_type() {
        VariantType::Struct => {
            if a.field_len() != b.field_len() {
                return None;
            }
            crate::structs::partial_cmp_by_field_names(
                a.field_len(),
                |i| a.name_at(i),
                |i| a.field_at(i),
                |i| b.name_at(i),
                |i| b.field_at(i),
                |name| b.field(name),
            )
        }
        VariantType::Tuple => {
            if a.field_len() != b.field_len() {
                return None;
            }
            for (i, field) in a.iter_fields().enumerate() {
                if let Some(field_value) = b.field_at(i) {
                    match field.value().reflect_partial_cmp(field_value) {
                        None => return None,
                        Some(core::cmp::Ordering::Equal) => continue,
                        Some(ord) => return Some(ord),
                    }
                }
                return None;
            }
            Some(core::cmp::Ordering::Equal)
        }
        _ => Some(core::cmp::Ordering::Equal),
    }
}

/// The default debug formatter for [`Enum`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
/// #[derive(Reflect)]
/// enum MyEnum {
///   A,
///   B (usize),
///   C {value: i32}
/// }
///
/// let my_enum: &dyn Reflect = &MyEnum::B(123);
/// println!("{:#?}", my_enum);
///
/// // Output:
///
/// // B (
/// //   123,
/// // )
/// ```
#[inline]
pub fn enum_debug(dyn_enum: &dyn Enum, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match dyn_enum.variant_type() {
        VariantType::Unit => f.write_str(dyn_enum.variant_name()),
        VariantType::Tuple => {
            let mut debug = f.debug_tuple(dyn_enum.variant_name());
            for field in dyn_enum.iter_fields() {
                debug.field(&field.value() as &dyn Debug);
            }
            debug.finish()
        }
        VariantType::Struct => {
            let mut debug = f.debug_struct(dyn_enum.variant_name());
            for field in dyn_enum.iter_fields() {
                debug.field(field.name().unwrap(), &field.value() as &dyn Debug);
            }
            debug.finish()
        }
    }
}
