//! [`CustomAttributes`], arbitrary reflected values attached to types, fields, and variants with
//! `#[reflect(@expr)]`.

use crate::Reflect;
use core::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
};

/// A custom attribute value, stored in [`CustomAttributes`].
pub type CustomAttribute = &'static (dyn Reflect + Send + Sync);

/// A collection of custom attributes for a type, field, or variant.
///
/// Custom attributes are arbitrary [`Reflect`] values, which are registered using
/// `#[reflect(@expr)]` with the [`Reflect` derive macro]. They are stored in the (`const`) type
/// information, so `expr` must be a constant expression whose value can be borrowed for `'static`
/// in a constant (ex: `Tooltip("hello")`, but not `String::from("hello")`).
///
/// Attributes are identified by their type. A collection can contain multiple attributes of the
/// same type, in which case the typed accessors (ex: [`CustomAttributes::get`]) return the first
/// one.
///
/// # Example
///
/// ```
/// # use bevy_reflect2::{kind::Kind, Reflect, Type};
/// #[derive(Reflect)]
/// struct Tooltip(&'static str);
///
/// #[derive(Reflect, PartialEq, Debug)]
/// struct Range(f32, f32);
///
/// #[derive(Reflect)]
/// #[reflect(@Tooltip("A slider"))]
/// struct Slider {
///     #[reflect(@Range(0.0, 1.0))]
///     value: f32,
/// }
///
/// let ty = Type::of::<Slider>();
/// assert_eq!(ty.get_attribute::<Tooltip>().unwrap().0, "A slider");
///
/// let Kind::Struct(struct_type) = ty.kind() else {
///     panic!("expected a struct");
/// };
/// let range = struct_type.fields()[0].get_attribute::<Range>().unwrap();
/// assert_eq!(*range, Range(0.0, 1.0));
/// ```
///
/// [`Reflect` derive macro]: derive@crate::Reflect
#[derive(Clone, Copy, Default)]
pub struct CustomAttributes {
    attributes: &'static [CustomAttribute],
}

impl CustomAttributes {
    /// An empty collection.
    pub const EMPTY: Self = Self::new(&[]);

    /// Creates a new collection containing `attributes`.
    #[inline]
    pub const fn new(attributes: &'static [CustomAttribute]) -> Self {
        Self { attributes }
    }

    /// Returns the custom attribute of type `T`, if there is one.
    pub fn get<T: Reflect>(&self) -> Option<&'static T> {
        self.get_by_id(TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Returns the custom attribute with the given [`TypeId`], if there is one.
    ///
    /// This is the dynamic equivalent of [`CustomAttributes::get`].
    pub fn get_by_id(&self, id: TypeId) -> Option<&'static dyn Reflect> {
        self.attributes
            .iter()
            .find(|attribute| attribute.ty().type_id() == id)
            .map(|attribute| *attribute as &'static dyn Reflect)
    }

    /// Returns `true` if this collection contains a custom attribute of type `T`.
    #[inline]
    pub fn contains<T: Reflect>(&self) -> bool {
        self.contains_by_id(TypeId::of::<T>())
    }

    /// Returns `true` if this collection contains a custom attribute with the given [`TypeId`].
    ///
    /// This is the dynamic equivalent of [`CustomAttributes::contains`].
    pub fn contains_by_id(&self, id: TypeId) -> bool {
        self.attributes
            .iter()
            .any(|attribute| attribute.ty().type_id() == id)
    }

    /// Iterates over the custom attributes, in declaration order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = CustomAttribute> + Clone + use<> {
        self.attributes.iter().copied()
    }

    /// Returns the custom attributes as a slice, in declaration order.
    #[inline]
    pub const fn as_slice(&self) -> &'static [CustomAttribute] {
        self.attributes
    }

    /// Returns the number of custom attributes.
    #[inline]
    pub const fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Returns `true` if there are no custom attributes.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

impl Debug for CustomAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.attributes).finish()
    }
}

/// Implements convenience methods for accessing the [`CustomAttributes`] of an item:
///
/// * `fn get_attribute<T: Reflect>(&self) -> Option<&'static T>`
/// * `fn get_attribute_by_id(&self, id: TypeId) -> Option<&'static dyn Reflect>`
/// * `fn has_attribute<T: Reflect>(&self) -> bool`
/// * `fn has_attribute_by_id(&self, id: TypeId) -> bool`
///
/// `$attributes` is an expression (using `$self`) that returns the [`CustomAttributes`], and `$term`
/// describes the item in the generated documentation.
macro_rules! impl_custom_attribute_methods {
    ($self:ident, $attributes:expr, $term:literal) => {
        #[doc = concat!("Returns the custom attribute of type `T` on this ", $term, ", if there is one.")]
        ///
        /// See [`CustomAttributes::get`](crate::attributes::CustomAttributes::get).
        #[inline]
        pub fn get_attribute<T: $crate::Reflect>(&$self) -> Option<&'static T> {
            $attributes.get::<T>()
        }

        #[doc = concat!("Returns the custom attribute on this ", $term, " with the given [`TypeId`](core::any::TypeId), if there is one.")]
        ///
        /// See [`CustomAttributes::get_by_id`](crate::attributes::CustomAttributes::get_by_id).
        #[inline]
        pub fn get_attribute_by_id(
            &$self,
            id: ::core::any::TypeId,
        ) -> Option<&'static dyn $crate::Reflect> {
            $attributes.get_by_id(id)
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a custom attribute of type `T`.")]
        #[inline]
        pub fn has_attribute<T: $crate::Reflect>(&$self) -> bool {
            $attributes.contains::<T>()
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a custom attribute with the given [`TypeId`](core::any::TypeId).")]
        #[inline]
        pub fn has_attribute_by_id(&$self, id: ::core::any::TypeId) -> bool {
            $attributes.contains_by_id(id)
        }
    };
}

pub(crate) use impl_custom_attribute_methods;
