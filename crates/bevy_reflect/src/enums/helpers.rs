use crate::{Enum, Reflect, ReflectRef, VariantType};
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

// TODO: Add serializable. How do we handle enums?
// pub fn enum_serialize<TEnum, S>(value: &TEnum, serializer: S) -> Result<S::Ok, S::Error>
// where
//     TEnum: Enum + ?Sized,
//     S: serde::Serializer,
// {
//
//
// }

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
        _ => Some(false),
    }
}
