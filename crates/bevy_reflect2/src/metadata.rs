//! [`Metadata`], arbitrary reflected values attached to types, fields, and variants with
//! `#[meta(expr)]` or `#[reflect(@expr)]`.

use crate::Reflect;
use core::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
};

/// A single metadata value, stored in [`Metadata`].
pub type MetadataValue = &'static (dyn Reflect + Send + Sync);

/// A collection of metadata values for a type, field, or variant.
///
/// Metadata values are arbitrary [`Reflect`] values, which are registered using
/// `#[reflect(@expr)]` with the [`Reflect` derive macro]. They can also be registered using
/// `#[meta(expr)]`, which is equivalent to `#[reflect(@expr)]`, but only accepts metadata
/// (so the `@` prefix is not needed). Multiple values can be registered at once (ex:
/// `#[meta(Tooltip("hello"), Step(0.5))]`). They are stored in the (`const`) type
/// information, so `expr` must be a constant expression whose value can be borrowed for `'static`
/// in a constant (ex: `Tooltip("hello")`, but not `String::from("hello")`).
///
/// Metadata values are identified by their type. A collection can contain multiple values of the
/// same type, in which case the typed accessors (ex: [`Metadata::get`]) return the first
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
///     // Equivalent to `#[reflect(@Range(0.0, 1.0))]`
///     #[meta(Range(0.0, 1.0))]
///     value: f32,
/// }
///
/// let ty = Type::of::<Slider>();
/// assert_eq!(ty.get_metadata::<Tooltip>().unwrap().0, "A slider");
///
/// let Kind::Struct(struct_type) = ty.kind() else {
///     panic!("expected a struct");
/// };
/// let range = struct_type.fields()[0].get_metadata::<Range>().unwrap();
/// assert_eq!(*range, Range(0.0, 1.0));
/// ```
///
/// [`Reflect` derive macro]: derive@crate::Reflect
#[derive(Clone, Copy, Default)]
pub struct Metadata {
    values: &'static [MetadataValue],
}

impl Metadata {
    /// An empty collection.
    pub const EMPTY: Self = Self::new(&[]);

    /// Creates a new collection containing `values`.
    #[inline]
    pub const fn new(values: &'static [MetadataValue]) -> Self {
        Self { values }
    }

    /// Returns the metadata value of type `T`, if there is one.
    pub fn get<T: Reflect>(&self) -> Option<&'static T> {
        self.get_by_id(TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Returns the metadata value with the given [`TypeId`], if there is one.
    ///
    /// This is the dynamic equivalent of [`Metadata::get`].
    pub fn get_by_id(&self, id: TypeId) -> Option<&'static dyn Reflect> {
        self.values
            .iter()
            .find(|value| value.ty().type_id() == id)
            .map(|value| *value as &'static dyn Reflect)
    }

    /// Returns `true` if this collection contains a metadata value of type `T`.
    #[inline]
    pub fn contains<T: Reflect>(&self) -> bool {
        self.contains_by_id(TypeId::of::<T>())
    }

    /// Returns `true` if this collection contains a metadata value with the given [`TypeId`].
    ///
    /// This is the dynamic equivalent of [`Metadata::contains`].
    pub fn contains_by_id(&self, id: TypeId) -> bool {
        self.values.iter().any(|value| value.ty().type_id() == id)
    }

    /// Iterates over the metadata values, in declaration order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = MetadataValue> + Clone + use<> {
        self.values.iter().copied()
    }

    /// Returns the metadata values as a slice, in declaration order.
    #[inline]
    pub const fn as_slice(&self) -> &'static [MetadataValue] {
        self.values
    }

    /// Returns the number of metadata values.
    #[inline]
    pub const fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` if there are no metadata values.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Debug for Metadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.values).finish()
    }
}

/// Implements convenience methods for accessing the [`Metadata`] of an item:
///
/// * `fn get_metadata<T: Reflect>(&self) -> Option<&'static T>`
/// * `fn get_metadata_by_id(&self, id: TypeId) -> Option<&'static dyn Reflect>`
/// * `fn has_metadata<T: Reflect>(&self) -> bool`
/// * `fn has_metadata_by_id(&self, id: TypeId) -> bool`
///
/// `$metadata` is an expression (using `$self`) that returns the [`Metadata`], and `$term`
/// describes the item in the generated documentation.
macro_rules! impl_metadata_methods {
    ($self:ident, $metadata:expr, $term:literal) => {
        #[doc = concat!("Returns the metadata value of type `T` on this ", $term, ", if there is one.")]
        ///
        /// See [`Metadata::get`](crate::metadata::Metadata::get).
        #[inline]
        pub fn get_metadata<T: $crate::Reflect>(&$self) -> Option<&'static T> {
            $metadata.get::<T>()
        }

        #[doc = concat!("Returns the metadata value on this ", $term, " with the given [`TypeId`](core::any::TypeId), if there is one.")]
        ///
        /// See [`Metadata::get_by_id`](crate::metadata::Metadata::get_by_id).
        #[inline]
        pub fn get_metadata_by_id(
            &$self,
            id: ::core::any::TypeId,
        ) -> Option<&'static dyn $crate::Reflect> {
            $metadata.get_by_id(id)
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a metadata value of type `T`.")]
        #[inline]
        pub fn has_metadata<T: $crate::Reflect>(&$self) -> bool {
            $metadata.contains::<T>()
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a metadata value with the given [`TypeId`](core::any::TypeId).")]
        #[inline]
        pub fn has_metadata_by_id(&$self, id: ::core::any::TypeId) -> bool {
            $metadata.contains_by_id(id)
        }
    };
}

pub(crate) use impl_metadata_methods;
