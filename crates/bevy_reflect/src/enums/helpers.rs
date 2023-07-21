use crate::equality_utility::{
    compare_structs, compare_tuple_structs, extract_info, get_type_info_pair,
};
use crate::utility::reflect_hasher;
use crate::{Enum, Reflect, ReflectRef, TypeInfo, VariantInfo, VariantType};
use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// Returns the `u64` hash of the given [enum](Enum).
#[inline]
pub fn enum_hash<TEnum: Enum>(value: &TEnum) -> Option<u64> {
    let mut hasher = reflect_hasher();

    match value.get_represented_type_info() {
        // Proxy case
        Some(info) => {
            let TypeInfo::Enum(info) = info else {
                return None;
            };

            let Some(variant) = info.variant(value.variant_name()) else {
                return None;
            };

            Hash::hash(&info.type_id(), &mut hasher);
            Hash::hash(&value.field_len(), &mut hasher);
            Hash::hash(variant.name(), &mut hasher);

            match variant {
                VariantInfo::Struct(info) => {
                    for field in info.iter() {
                        if field.meta().skip_hash {
                            continue;
                        }

                        if let Some(value) = value.field(field.name()) {
                            Hash::hash(&value.reflect_hash()?, &mut hasher);
                        }
                    }
                }
                VariantInfo::Tuple(info) => {
                    for field in info.iter() {
                        if field.meta().skip_hash {
                            continue;
                        }

                        if let Some(value) = value.field_at(field.index()) {
                            Hash::hash(&value.reflect_hash()?, &mut hasher);
                        }
                    }
                }
                VariantInfo::Unit(_) => {}
            }
        }
        // Dynamic case
        None => {
            Hash::hash(&TypeId::of::<TEnum>(), &mut hasher);
            Hash::hash(&value.field_len(), &mut hasher);
            Hash::hash(&value.variant_name(), &mut hasher);

            for field in value.iter_fields() {
                hasher.write_u64(field.value().reflect_hash()?);
            }
        }
    }

    Some(hasher.finish())
}

/// Compares an [`Enum`] with a [`Reflect`] value.
///
/// Returns `Some(true)` if and only if all of the following are true:
/// - `b` is an enum;
/// - `b` is the same variant as `a`;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values[^1]
///
/// Returns `None` if the comparison cannot be performed.
///
/// [^1]: If a field is marked with `#[reflect(skip_partial_eq)]`, then it will be skipped
///       unless the corresponding field in `b` is not marked with `#[reflect(skip_partial_eq)]`,
///       in which case the comparison will return `Some(false)`.
#[inline]
pub fn enum_partial_eq<TEnum: Enum>(a: &TEnum, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::Enum(b) = b.reflect_ref()  else {
        return Some(false);
    };

    if a.variant_name() != b.variant_name() {
        return Some(false);
    }

    if !a.is_variant(b.variant_type()) {
        return Some(false);
    }

    let variant_name = a.variant_name();

    let (info_a, info_b) = get_type_info_pair(a.as_reflect(), b.as_reflect());
    let info_a = extract_info!(info_a, TypeInfo::Enum(info) => info.variant(variant_name));
    let info_b = extract_info!(info_b, TypeInfo::Enum(info) => info.variant(variant_name));

    match a.variant_type() {
        VariantType::Struct => {
            let info_a = extract_info!(info_a, VariantInfo::Struct(info) => Some(info));
            let info_b = extract_info!(info_b, VariantInfo::Struct(info) => Some(info));

            compare_structs!(a, b, info_a, info_b, accessor=.value)
        }
        VariantType::Tuple => {
            let info_a = extract_info!(info_a, VariantInfo::Tuple(info) => Some(info));
            let info_b = extract_info!(info_b, VariantInfo::Tuple(info) => Some(info));

            compare_tuple_structs!(a, b, info_a, info_b, accessor=.field_at)
        }
        VariantType::Unit => Some(true),
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
