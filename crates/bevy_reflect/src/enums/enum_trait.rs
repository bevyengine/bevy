use crate::attributes::{impl_custom_attribute_methods, CustomAttributes};
use crate::{DynamicEnum, Reflect, TypePath, TypePathTable, VariantInfo, VariantType};
use bevy_utils::HashMap;
use std::any::{Any, TypeId};
use std::slice::Iter;
use std::sync::Arc;

/// A trait used to power [enum-like] operations via [reflection].
///
/// This allows enums to be processed and modified dynamically at runtime without
/// necessarily knowing the actual type.
/// Enums are much more complex than their struct counterparts.
/// As a result, users will need to be mindful of conventions, considerations,
/// and complications when working with this trait.
///
/// # Variants
///
/// An enum is a set of choices called _variants_.
/// An instance of an enum can only exist as one of these choices at any given time.
/// Consider Rust's [`Option<T>`]. It's an enum with two variants: [`None`] and [`Some`].
/// If you're `None`, you can't be `Some` and vice versa.
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
/// can contain one or more fields.
/// The fields in a tuple variant is defined by their _order_ within the variant.
/// Index `0` represents the first field in the variant and so on.
/// Fields in struct variants (excluding tuple structs), on the other hand, are
/// represented by a _name_.
///
/// # Implementation
///
/// > 💡 This trait can be automatically implemented using [`#[derive(Reflect)]`](derive@crate::Reflect)
/// > on an enum definition.
///
/// Despite the fact that enums can represent multiple states, traits only exist in one state
/// and must be applied to the entire enum rather than a particular variant.
/// Because of this limitation, the [`Enum`] trait must not only _represent_ any of the
/// three variant types, but also define the _methods_ for all three as well.
///
/// What does this mean? It means that even though a unit variant contains no fields, a
/// representation of that variant using the [`Enum`] trait will still contain methods for
/// accessing fields!
/// Again, this is to account for _all three_ variant types.
///
/// We recommend using the built-in [`#[derive(Reflect)]`](derive@crate::Reflect) macro to automatically handle all the
/// implementation details for you.
/// However, if you _must_ implement this trait manually, there are a few things to keep in mind...
///
/// ## Field Order
///
/// While tuple variants identify their fields by the order in which they are defined, struct
/// variants identify fields by their name.
/// However, both should allow access to fields by their defined order.
///
/// The reason all fields, regardless of variant type, need to be accessible by their order is
/// due to field iteration.
/// We need a way to iterate through each field in a variant, and the easiest way of achieving
/// that is through the use of field order.
///
/// The derive macro adds proper struct variant handling for [`Enum::index_of`], [`Enum::name_at`]
/// and [`Enum::field_at[_mut]`](Enum::field_at) methods.
/// The first two methods are __required__ for all struct variant types.
/// By convention, implementors should also handle the last method as well, but this is not
/// a strict requirement.
///
/// ## Field Names
///
/// Implementors may choose to handle [`Enum::index_of`], [`Enum::name_at`], and
/// [`Enum::field[_mut]`](Enum::field) for tuple variants by considering stringified `usize`s to be
/// valid names (such as `"3"`).
/// This isn't wrong to do, but the convention set by the derive macro is that it isn't supported.
/// It's preferred that these strings be converted to their proper `usize` representations and
/// the [`Enum::field_at[_mut]`](Enum::field_at) methods be used instead.
///
/// [enum-like]: https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html
/// [reflection]: crate
/// [`None`]: Option<T>::None
/// [`Some`]: Option<T>::Some
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
    /// The index of the current variant.
    fn variant_index(&self) -> usize;
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
        format!("{}::{}", self.reflect_type_path(), self.variant_name())
    }
}

/// A container for compile-time enum info, used by [`TypeInfo`](crate::TypeInfo).
#[derive(Clone, Debug)]
pub struct EnumInfo {
    type_path: TypePathTable,
    type_id: TypeId,
    variants: Box<[VariantInfo]>,
    variant_names: Box<[&'static str]>,
    variant_indices: HashMap<&'static str, usize>,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl EnumInfo {
    /// Create a new [`EnumInfo`].
    ///
    /// # Arguments
    ///
    /// * `variants`: The variants of this enum in the order they are defined
    ///
    pub fn new<TEnum: Enum + TypePath>(variants: &[VariantInfo]) -> Self {
        let variant_indices = variants
            .iter()
            .enumerate()
            .map(|(index, variant)| (variant.name(), index))
            .collect::<HashMap<_, _>>();

        let variant_names = variants.iter().map(|variant| variant.name()).collect();

        Self {
            type_path: TypePathTable::of::<TEnum>(),
            type_id: TypeId::of::<TEnum>(),
            variants: variants.to_vec().into_boxed_slice(),
            variant_names,
            variant_indices,
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this enum.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this enum.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
    }

    /// A slice containing the names of all variants in order.
    pub fn variant_names(&self) -> &[&'static str] {
        &self.variant_names
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
        format!("{}::{name}", self.type_path())
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

    /// A representation of the type path of the value.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path
    }

    /// The [stable, full type path] of the value.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.type_path_table().path()
    }

    /// The [`TypeId`] of the enum.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the enum type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The docstring of this enum, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "enum");
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
    type Item = VariantField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.container.variant_type() {
            VariantType::Unit => None,
            VariantType::Tuple => Some(VariantField::Tuple(self.container.field_at(self.index)?)),
            VariantType::Struct => {
                let name = self.container.name_at(self.index)?;
                Some(VariantField::Struct(name, self.container.field(name)?))
            }
        };
        self.index += value.is_some() as usize;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.container.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for VariantFieldIter<'a> {}

pub enum VariantField<'a> {
    Struct(&'a str, &'a dyn Reflect),
    Tuple(&'a dyn Reflect),
}

impl<'a> VariantField<'a> {
    pub fn name(&self) -> Option<&'a str> {
        if let Self::Struct(name, ..) = self {
            Some(*name)
        } else {
            None
        }
    }

    pub fn value(&self) -> &'a dyn Reflect {
        match *self {
            Self::Struct(_, value) | Self::Tuple(value) => value,
        }
    }
}

// Tests that need access to internal fields have to go here rather than in mod.rs
#[cfg(test)]
mod tests {
    use crate as bevy_reflect;
    use crate::*;

    #[derive(Reflect, Debug, PartialEq)]
    enum MyEnum {
        A,
        B(usize, i32),
        C { foo: f32, bar: bool },
    }
    #[test]
    fn next_index_increment() {
        // unit enums always return none, so index should stay at 0
        let unit_enum = MyEnum::A;
        let mut iter = unit_enum.iter_fields();
        let size = iter.len();
        for _ in 0..2 {
            assert!(iter.next().is_none());
            assert_eq!(size, iter.index);
        }
        // tuple enums we iter over each value (unnamed fields), stop after that
        let tuple_enum = MyEnum::B(0, 1);
        let mut iter = tuple_enum.iter_fields();
        let size = iter.len();
        for _ in 0..2 {
            let prev_index = iter.index;
            assert!(iter.next().is_some());
            assert_eq!(prev_index, iter.index - 1);
        }
        for _ in 0..2 {
            assert!(iter.next().is_none());
            assert_eq!(size, iter.index);
        }

        // struct enums, we iterate over each field in the struct
        let struct_enum = MyEnum::C {
            foo: 0.,
            bar: false,
        };
        let mut iter = struct_enum.iter_fields();
        let size = iter.len();
        for _ in 0..2 {
            let prev_index = iter.index;
            assert!(iter.next().is_some());
            assert_eq!(prev_index, iter.index - 1);
        }
        for _ in 0..2 {
            assert!(iter.next().is_none());
            assert_eq!(size, iter.index);
        }
    }
}
