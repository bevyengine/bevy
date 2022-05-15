use crate::{DynamicEnum, Reflect, ReflectRef, Struct, Tuple, VariantInfo, VariantType};
use bevy_utils::HashMap;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::slice::Iter;

/// A trait representing a [reflected] enum.
///
/// This allows enums to be processed and modified dynamically at runtime without
/// necessarily knowing the actual type. Enums, unlike their struct counterparts,
/// are a lot more complex. Users will need to be mindful of conventions,
/// considerations, and complications when working with this trait.
///
/// # Variants
///
/// An enum is a set of choices called _variants_. An instance of an enum can only
/// exist as one of these choices at any given time. Consider Rust's [`Option<T>`].
/// It's an enum with two variants: [`None`] and [`Some`]. If you're `None`, you can't
/// be `Some` and vice versa.
///
/// > ⚠️ __This is very important:__
/// > The [`Enum`] trait represents an enum _as one of its variants_.
/// > It does not represent the entire enum since that's not true to how enums work.
///
/// Variants come in a few [flavors](VariantType):
///
/// | Variant Type | Syntax                         |
/// | ------------ | ------------------------------ |
/// | Unit         | `MyEnum::Foo`                  |
/// | Tuple        | `MyEnum::Foo( i32, i32 )`      |
/// | Struct       | `MyEnum::Foo{ value: String }` |
///
/// As you can see, a unit variant contains no fields, while tuple and struct variants
/// can contain one or more fields. The fields in a tuple variant is defined by their
/// _order_ within the variant. Index `0` represents the first field in the variant and
/// so on. Fields in struct variants, on the other hand, are represented by a _name_.
///
/// # Implementation
///
/// > 💡 This trait can be automatically implemented using the [`Reflect`] derive macro
/// > on an enum definition.
///
/// Despite the fact that enums can represent multiple states, traits only exist in one state
/// and must be applied to the entire enum rather than a particular variant. Because of this
/// limitation, the [`Enum`] trait must not only _represent- any of the three variant types,
/// but also define the _methods_ for all three as well.
///
/// What does this mean? It means that even though a unit variant contains no fields, a
/// representation of that variant using the [`Enum`] trait will still contain methods for
/// accessing fields! Again, this is to account for _all three_ variant types.
///
/// We recommend using the built-in [`Reflect`] derive macro to automatically handle all the
/// implementation details for you. However, if you _must_ implement this trait manually, there
/// are a few things to keep in mind...
///
/// ## Field Order
///
/// While tuple variants identify their fields by the order in which they are defined, struct
/// variants identify fields by their name. However, both should allow access to fields by their
/// defined order.
///
/// The reason all fields, regardless of variant type, need to be accessible by their order is
/// due to field iteration. We need a way to iterate through each field in a variant, and the
/// easiest way of achieving that is through the use of field order.
///
/// The derive macro adds proper struct variant handling for [`Enum::index_of`], [`Enum::name_at`]
/// and [`Enum::field_at[_mut]`](Enum::field_at) methods. The first two methods are __required__ for
/// all struct variant types. By convention, implementors should also handle the last method as well,
/// but this is not a strict requirement.
///
/// ## Field Names
///
/// Implementors may choose to handle [`Enum::index_of`], [`Enum::name_at`], and
/// [`Enum::field[_mut]`](Enum::field) for tuple variants by considering stringified `usize`s to be
/// valid names (such as `"3"`). This isn't wrong to do, but the convention set by the derive macro
/// is that it isn't supported. It's preferred that these strings be converted to their proper `usize`
/// representations and the [`Enum::field_at[_mut]`](Enum::field_at) methods be used instead.
///
/// [reflected]: crate
/// [`None`]: core::option::Option<T>::None
/// [`Some`]: core::option::Option<T>::Some
/// [`Reflect`]: bevy_reflect_derive::Reflect
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
    // Clones the enum into a [`DynamicEnum`].
    fn clone_dynamic(&self) -> DynamicEnum;
    /// Returns true if the current variant's type matches the given one.
    fn is_variant(&self, variant_type: VariantType) -> bool {
        self.variant_type() == variant_type
    }
    /// Returns the full path to the current variant.
    fn variant_path(&self) -> String {
        format!("{}::{}", self.type_name(), self.variant_name())
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

    /// Returns the full path to the given variant.
    ///
    /// This does _not_ check if the given variant exists.
    pub fn variant_path(&self, name: &str) -> String {
        format!("{}::{}", self.id().type_name(), name)
    }

    /// Checks if a variant with the given name exists within this enum.
    pub fn contains_variant(&self, name: &str) -> bool {
        self.variant_indices.contains_key(name)
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
        let value = match self.container.variant_type() {
            VariantType::Unit => None,
            VariantType::Tuple => self.container.field_at(self.index),
            VariantType::Struct => {
                let name = self.container.name_at(self.index)?;
                self.container.field(name)
            }
        };
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.container.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for VariantFieldIter<'a> {}
