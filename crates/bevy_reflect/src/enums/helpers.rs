use crate::{Enum, Reflect, ReflectRef, VariantType};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// Returns the `u64` hash of the given [enum](Enum).
#[inline]
pub fn enum_hash<TEnum: Enum>(value: &TEnum) -> Option<u64> {
    let mut hasher = crate::ReflectHasher::default();
    std::any::Any::type_id(value).hash(&mut hasher);
    value.variant_name().hash(&mut hasher);
    value.variant_type().hash(&mut hasher);
    for field in value.iter_fields() {
        hasher.write_u64(field.value().reflect_hash()?);
    }
    Some(hasher.finish())
}

/// Compares an [`Enum`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is an enum;
/// - `b` is the same variant as `a`;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values.
#[inline]
pub fn enum_partial_eq<TEnum: Enum>(a: &TEnum, b: &dyn Reflect) -> Option<bool> {
    // Both enums?
    let b = if let ReflectRef::Enum(e) = b.reflect_ref() {
        e
    } else {
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
pub fn enum_debug(dyn_enum: &dyn Enum, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
