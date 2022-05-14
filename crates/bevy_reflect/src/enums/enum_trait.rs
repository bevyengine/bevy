use crate::{Reflect, ReflectRef, Struct, Tuple, VariantInfo, VariantType};
use bevy_utils::HashMap;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::slice::Iter;

pub trait Enum: Reflect {
    /// Returns a reference to the value of the field (in the current variant) with the given name.
    ///
    /// For non-[`VariantType::Struct`] variants, this should return `None`.
    fn field(&self, name: &str) -> Option<&dyn Reflect>;
    /// Returns a reference to the value of the field (in the current variant) at the given index.
    fn field_at(&self, index: usize) -> Option<&dyn Reflect>;
    /// Returns a mutable reference to the value of the field (in the current variant) with the given name.
    ///
    /// For non-[`VariantType::Struct`] variants, this should return `None`.
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect>;
    /// Returns a mutable reference to the value of the field (in the current variant) at the given index.
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    /// Returns the index of the field (in the current variant) with the given name.
    ///
    /// For non-[`VariantType::Struct`] variants, this should return `None`.
    fn index_of(&self, name: &str) -> Option<usize>;
    /// Returns the name of the field (in the current variant) with the given index.
    ///
    /// For non-[`VariantType::Struct`] variants, this should return `None`.
    fn name_at(&self, index: usize) -> Option<&str>;
    /// Returns an iterator over the values of the current variant's fields.
    fn iter_fields(&self) -> VariantFieldIter;
    /// Returns the number of fields in the current variant.
    fn field_len(&self) -> usize;
    /// The name of the current variant.
    fn variant_name(&self) -> &str;
    /// The type of the current variant.
    fn variant_type(&self) -> VariantType;
    /// Returns true if the current variant's type matches the given one.
    fn is_variant(&self, variant_type: VariantType) -> bool {
        self.variant_type() == variant_type
    }
}

/// A container for compile-time enum info.
#[derive(Clone, Debug)]
pub struct EnumInfo {
    type_name: &'static str,
    type_id: TypeId,
    variants: Box<[VariantInfo]>,
    variant_indices: HashMap<Cow<'static, str>, usize>,
}

impl EnumInfo {
    /// Create a new [`EnumInfo`].
    ///
    /// # Arguments
    ///
    /// * `variants`: The variants of this enum in the order they are defined
    ///
    pub fn new<TEnum: Enum>(variants: &[VariantInfo]) -> Self {
        let variant_indices = variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                let name = variant.name().clone();
                (name, index)
            })
            .collect::<HashMap<_, _>>();

        Self {
            type_name: std::any::type_name::<TEnum>(),
            type_id: TypeId::of::<TEnum>(),
            variants: variants.to_vec().into_boxed_slice(),
            variant_indices,
        }
    }

    /// Get a variant with the given name.
    pub fn variant(&self, name: &str) -> Option<&VariantInfo> {
        self.variant_indices
            .get(name)
            .map(|index| &self.variants[*index])
    }

    /// Get a variant at the given index.
    pub fn variant_at(&self, index: usize) -> Option<&VariantInfo> {
        self.variants.get(index)
    }

    /// Get the index of the variant with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.variant_indices.get(name).copied()
    }

    /// Iterate over the variants of this enum.
    pub fn iter(&self) -> Iter<'_, VariantInfo> {
        self.variants.iter()
    }

    /// The number of variants in this enum.
    pub fn variant_len(&self) -> usize {
        self.variants.len()
    }

    /// The [type name] of the enum.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the enum.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the enum type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// An iterator over the fields in the current enum variant.
pub struct VariantFieldIter<'a> {
    container: &'a dyn Enum,
    index: usize,
}

impl<'a> VariantFieldIter<'a> {
    pub fn new(container: &'a dyn Enum) -> Self {
        Self {
            container,
            index: 0,
        }
    }
}

impl<'a> Iterator for VariantFieldIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.container.field_at(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.container.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for VariantFieldIter<'a> {}

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
    let enum_b = if let ReflectRef::Enum(e) = b.reflect_ref() {
        e
    } else {
        return Some(false);
    };

    // Same variant name?
    if a.variant_name() != enum_b.variant_name() {
        return Some(false);
    }

    // Same variant type?
    if !a.is_variant(enum_b.variant_type()) {
        return Some(false);
    }

    match a.variant_type() {
        VariantType::Struct => {
            // Same struct fields?
            for (i, value) in a.iter_fields().enumerate() {
                let field_name = a.name_at(i).unwrap();
                if let Some(field_value) = enum_b.field(field_name) {
                    if let Some(false) | None = field_value.reflect_partial_eq(value) {
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
            for (i, value) in a.iter_fields().enumerate() {
                if let Some(field_value) = enum_b.field_at(i) {
                    if let Some(false) | None = field_value.reflect_partial_eq(value) {
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
