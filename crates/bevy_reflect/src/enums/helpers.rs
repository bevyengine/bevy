use crate::{
    dynamic::{DynamicEnum, DynamicValue, DynamicValueRef, DynamicVariantField},
    enums::{Enum, VariantField, VariantType},
    utility::reflect_hasher,
    Reflect, ReflectRef,
};
use core::{
    cmp::Ordering,
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

/// Returns the `u64` hash of the given [`DynamicEnum`].
pub(crate) fn dynamic_enum_hash(value: &DynamicEnum) -> Option<u64> {
    let mut hasher = reflect_hasher();
    // TODO: This type_id is **not** correct.
    core::any::Any::type_id(value).hash(&mut hasher);
    value.variant_name().hash(&mut hasher);
    value.variant_type().hash(&mut hasher);
    for field in value.iter_fields() {
        hasher.write_u64(field.value().dynamic_hash()?);
    }
    Some(hasher.finish())
}

pub(crate) trait EnumLike {
    fn variant_name(&self) -> &str;

    fn variant_type(&self) -> VariantType;

    fn field_len(&self) -> usize;

    fn field(&self, name: &str) -> Option<DynamicValueRef<'_>>;

    fn field_at(&self, index: usize) -> Option<DynamicValueRef<'_>>;

    fn name_at(&self, index: usize) -> Option<&str>;

    fn iter_fields(&self) -> impl Iterator<Item = DynamicVariantField<'_>>;
}

impl EnumLike for DynamicEnum {
    fn variant_name(&self) -> &str {
        self.variant_name()
    }

    fn variant_type(&self) -> VariantType {
        self.variant_type()
    }

    fn field_len(&self) -> usize {
        self.field_len()
    }

    fn field(&self, name: &str) -> Option<DynamicValueRef<'_>> {
        self.field(name).map(DynamicValue::as_ref)
    }

    fn field_at(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.field_at(index).map(DynamicValue::as_ref)
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.name_at(index)
    }

    fn iter_fields(&self) -> impl Iterator<Item = DynamicVariantField<'_>> {
        self.iter_fields()
    }
}

impl EnumLike for dyn Enum {
    fn variant_name(&self) -> &str {
        self.variant_name()
    }

    fn variant_type(&self) -> VariantType {
        self.variant_type()
    }

    fn field_len(&self) -> usize {
        self.field_len()
    }

    fn field(&self, name: &str) -> Option<DynamicValueRef<'_>> {
        self.field(name).map(|value| DynamicValueRef::from(value))
    }

    fn field_at(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.field_at(index)
            .map(|value| DynamicValueRef::from(value))
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.name_at(index)
    }

    fn iter_fields(&self) -> impl Iterator<Item = DynamicVariantField<'_>> {
        self.iter_fields().map(|value| match value {
            VariantField::Struct(name, value) => {
                DynamicVariantField::Struct(name, DynamicValueRef::from(value))
            }
            VariantField::Tuple(value) => DynamicVariantField::Tuple(DynamicValueRef::from(value)),
        })
    }
}

/// Compares an [`Enum`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is an enum;
/// - `b` is the same variant as `a`;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values.
#[inline(never)]
pub fn enum_partial_eq(a: &dyn Enum, b: &dyn Reflect) -> Option<bool> {
    // Both enums?
    let ReflectRef::Enum(b) = b.reflect_ref() else {
        return Some(false);
    };
    enum_like_partial_eq(a, b)
}

/// Same as above, but for any two [`EnumLike`] types.
pub(crate) fn enum_like_partial_eq<L: EnumLike + ?Sized, R: EnumLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<bool> {
    // Same variant name?
    if a.variant_name() != b.variant_name() {
        return Some(false);
    }

    // Same variant type?
    if a.variant_type() != b.variant_type() {
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
                    if let Some(false) | None = field_value.dynamic_partial_eq(field.value()) {
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
                    if let Some(false) | None = field_value.dynamic_partial_eq(field.value()) {
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
pub fn enum_partial_cmp(a: &dyn Enum, b: &dyn Reflect) -> Option<Ordering> {
    // Both enums?
    let ReflectRef::Enum(b) = b.reflect_ref() else {
        return None;
    };
    enum_like_partial_cmp(a, b)
}

/// Same as above, but for any two [`EnumLike`] types.
pub(crate) fn enum_like_partial_cmp<L: EnumLike + ?Sized, R: EnumLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<Ordering> {
    // Same variant name?
    if a.variant_name() != b.variant_name() {
        // Different variant names.
        // Ordering by variant index here can result in inconsistencies with
        // partial_eq when comparing between two different concrete enums,
        // so we simply return None here
        return None;
    }

    // Same variant type?
    if a.variant_type() != b.variant_type() {
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
                    match field.value().dynamic_partial_cmp(field_value) {
                        None => return None,
                        Some(Ordering::Equal) => continue,
                        Some(ord) => return Some(ord),
                    }
                }
                return None;
            }
            Some(Ordering::Equal)
        }
        _ => Some(Ordering::Equal),
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
