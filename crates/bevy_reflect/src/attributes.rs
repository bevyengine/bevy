//! Types and functions for creating, manipulating and querying [`CustomAttributes`].

use crate::Reflect;
use alloc::boxed::Box;
use bevy_platform::sync::Arc;
use bevy_utils::TypeIdMap;
use core::{
    any::TypeId,
    fmt::{Debug, Formatter},
};

/// A collection of custom attributes for a type, field, or variant.
///
/// These attributes can be created with the [`Reflect` derive macro], or with
/// [`CustomAttributesBuilder`].
///
/// Attributes are stored by their [`TypeId`].
/// Because of this, there can only be one attribute per type.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{Reflect, Typed, TypeInfo};
/// use core::ops::RangeInclusive;
/// #[derive(Reflect)]
/// struct Slider {
///   #[reflect(@RangeInclusive::<f32>::new(0.0, 1.0))]
///   value: f32
/// }
///
/// let TypeInfo::Struct(info) = <Slider as Typed>::type_info() else {
///   panic!("expected struct info");
/// };
///
/// let range = info.field("value").unwrap().get_attribute::<RangeInclusive<f32>>().unwrap();
/// assert_eq!(0.0..=1.0, *range);
/// ```
///
/// [`Reflect` derive macro]: derive@crate::Reflect
#[derive(Default, Clone)]
pub struct CustomAttributes {
    attributes: Option<Arc<TypeIdMap<CustomAttribute>>>,
}

impl CustomAttributes {
    fn new(attributes: TypeIdMap<CustomAttribute>) -> Self {
        Self {
            attributes: if attributes.is_empty() {
                None
            } else {
                Some(Arc::new(attributes))
            },
        }
    }

    /// Returns `true` if this collection contains a custom attribute of the specified type.
    pub fn contains<T: Reflect>(&self) -> bool {
        self.attributes
            .as_ref()
            .is_some_and(|a| a.contains_key(&TypeId::of::<T>()))
    }

    /// Returns `true` if this collection contains a custom attribute with the specified [`TypeId`].
    pub fn contains_by_id(&self, id: TypeId) -> bool {
        self.attributes
            .as_ref()
            .is_some_and(|a| a.contains_key(&id))
    }

    /// Gets a custom attribute by type.
    pub fn get<T: Reflect>(&self) -> Option<&T> {
        self.attributes
            .as_ref()
            .and_then(|a| a.get(&TypeId::of::<T>())?.value::<T>())
    }

    /// Gets a custom attribute by its [`TypeId`].
    pub fn get_by_id(&self, id: TypeId) -> Option<&dyn Reflect> {
        self.attributes
            .as_ref()
            .and_then(|a| a.get(&id))
            .map(CustomAttribute::reflect_value)
    }

    /// Returns an iterator over all custom attributes.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeId, &dyn Reflect)> {
        self.attributes.iter().flat_map(|attributes| {
            attributes
                .iter()
                .map(|(key, value)| (key, value.reflect_value()))
        })
    }

    /// Returns the number of custom attributes in this collection.
    pub fn len(&self) -> usize {
        self.attributes.as_ref().map(|a| a.len()).unwrap_or(0)
    }

    /// Returns `true` if this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.attributes.is_none()
    }
}

impl Debug for CustomAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if let Some(attributes) = &self.attributes {
            f.debug_set().entries(attributes.values()).finish()
        } else {
            f.debug_set().finish()
        }
    }
}

struct CustomAttribute {
    value: Box<dyn Reflect>,
}

impl CustomAttribute {
    /// Creates a new [`CustomAttribute`] containing `value`.
    pub fn new<T: Reflect>(value: T) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    /// Returns a reference to the attribute's value if it is of type `T`, or [`None`] if not.
    pub fn value<T: Reflect>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }

    /// Returns a reference to the attribute's value as a [`Reflect`] trait object.
    pub fn reflect_value(&self) -> &dyn Reflect {
        &*self.value
    }
}

impl Debug for CustomAttribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.value.debug(f)
    }
}

/// Implements methods for accessing custom attributes.
///
/// Implements the following methods:
///
/// * `fn custom_attributes(&self) -> &CustomAttributes`
/// * `fn get_attribute<T: Reflect>(&self) -> Option<&T>`
/// * `fn get_attribute_by_id(&self, id: TypeId) -> Option<&dyn Reflect>`
/// * `fn has_attribute<T: Reflect>(&self) -> bool`
/// * `fn has_attribute_by_id(&self, id: TypeId) -> bool`
///
/// # Params
///
/// * `$self` - The name of the variable containing the custom attributes (usually `self`).
/// * `$attributes` - The name of the field containing the [`CustomAttributes`].
/// * `$term` - (Optional) The term used to describe the type containing the custom attributes.
///   This is purely used to generate better documentation. Defaults to `"item"`.
macro_rules! impl_custom_attribute_methods {
    ($self:ident . $attributes:ident, $term:literal) => {
        $crate::attributes::impl_custom_attribute_methods!($self, &$self.$attributes, "item");
    };
    ($self:ident, $attributes:expr, $term:literal) => {
        #[doc = concat!("Returns the custom attributes for this ", $term, ".")]
        pub fn custom_attributes(&$self) -> &$crate::attributes::CustomAttributes {
            $attributes
        }

        /// Gets a custom attribute by type.
        ///
        /// For dynamically accessing an attribute, see [`get_attribute_by_id`](Self::get_attribute_by_id).
        pub fn get_attribute<T: $crate::Reflect>(&$self) -> Option<&T> {
            $self.custom_attributes().get::<T>()
        }

        /// Gets a custom attribute by its [`TypeId`](core::any::TypeId).
        ///
        /// This is the dynamic equivalent of [`get_attribute`](Self::get_attribute).
        pub fn get_attribute_by_id(&$self, id: ::core::any::TypeId) -> Option<&dyn $crate::Reflect> {
            $self.custom_attributes().get_by_id(id)
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a custom attribute of the specified type.")]
        #[doc = "\n\nFor dynamically checking if an attribute exists, see [`has_attribute_by_id`](Self::has_attribute_by_id)."]
        pub fn has_attribute<T: $crate::Reflect>(&$self) -> bool {
            $self.custom_attributes().contains::<T>()
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a custom attribute with the specified [`TypeId`](::core::any::TypeId).")]
        #[doc = "\n\nThis is the dynamic equivalent of [`has_attribute`](Self::has_attribute)"]
        pub fn has_attribute_by_id(&$self, id: ::core::any::TypeId) -> bool {
            $self.custom_attributes().contains_by_id(id)
        }
    };
}

/// Builder for [`CustomAttributes`].
///
/// ```
/// # use bevy_reflect::attributes::CustomAttributesBuilder;
/// let custom_attributes = CustomAttributesBuilder::new()
///     .attribute("my attribute")
///     .attribute(123)
///     .build();
/// ```
#[derive(Default)]
pub struct CustomAttributesBuilder {
    attributes: TypeIdMap<CustomAttribute>,
}

impl CustomAttributesBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single attribute to the builder.
    pub fn attribute<T: Reflect>(self, value: T) -> Self {
        self.attribute_erased(TypeId::of::<T>(), CustomAttribute::new(value))
    }

    // Erased version of `attribute` with inlining disabled. This reduces
    // monomorphization costs, and avoids excessive inlining in cold generated
    // code.
    #[inline(never)]
    fn attribute_erased(mut self, type_id: TypeId, value: CustomAttribute) -> Self {
        self.attributes.insert(type_id, value);
        self
    }

    /// Consumes the builder, returning the final [`CustomAttributes`].
    pub fn build(self) -> CustomAttributes {
        CustomAttributes::new(self.attributes)
    }
}

pub(crate) use impl_custom_attribute_methods;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{enums::VariantInfo, type_info::Typed, TypeInfo};
    use alloc::{format, string::String};
    use core::ops::RangeInclusive;

    #[derive(Reflect, PartialEq, Debug)]
    struct Tooltip(String);

    impl Tooltip {
        fn new(value: impl Into<String>) -> Self {
            Self(value.into())
        }
    }

    #[test]
    fn should_get_custom_attribute() {
        let attributes = CustomAttributesBuilder::new().attribute(0.0..=1.0).build();

        let value = attributes.get::<RangeInclusive<f64>>().unwrap();
        assert_eq!(&(0.0..=1.0), value);
    }

    #[test]
    fn should_get_custom_attribute_dynamically() {
        let attributes = CustomAttributesBuilder::new()
            .attribute(String::from("Hello, World!"))
            .build();

        let value = attributes.get_by_id(TypeId::of::<String>()).unwrap();
        assert!(value
            .reflect_partial_eq(&String::from("Hello, World!"))
            .unwrap());
    }

    #[test]
    fn should_iterate_custom_attribute() {
        let empty_attributes = CustomAttributesBuilder::new().build();

        assert!(empty_attributes.iter().next().is_none());

        let attributes = CustomAttributesBuilder::new()
            .attribute(1i32)
            .attribute("string")
            .build();

        let mut iter = attributes.iter();

        let (type_id, reflected) = iter.next().unwrap();

        assert_eq!(TypeId::of::<i32>(), *type_id);
        assert_eq!(1i32, *reflected.downcast_ref::<i32>().unwrap());

        let (type_id, reflected) = iter.next().unwrap();

        assert_eq!(TypeId::of::<&str>(), *type_id);
        assert_eq!("string", *reflected.downcast_ref::<&str>().unwrap());

        assert!(iter.next().is_none());
    }

    #[test]
    fn should_debug_custom_attributes() {
        let attributes = CustomAttributesBuilder::new().build();

        let debug = format!("{attributes:?}");

        assert_eq!(r#"{}"#, debug);

        let attributes = CustomAttributesBuilder::new()
            .attribute("My awesome custom attribute!")
            .build();

        let debug = format!("{attributes:?}");

        assert_eq!(r#"{"My awesome custom attribute!"}"#, debug);

        #[derive(Reflect)]
        struct Foo {
            value: i32,
        }

        let attributes = CustomAttributesBuilder::new()
            .attribute(Foo { value: 42 })
            .build();

        let debug = format!("{attributes:?}");

        assert_eq!(
            r#"{bevy_reflect::attributes::tests::Foo { value: 42 }}"#,
            debug
        );
    }

    #[test]
    fn should_derive_custom_attributes_on_struct_container() {
        #[derive(Reflect)]
        #[reflect(@Tooltip::new("My awesome custom attribute!"))]
        struct Slider {
            value: f32,
        }

        let TypeInfo::Struct(info) = Slider::type_info() else {
            panic!("expected struct info");
        };

        let tooltip = info.get_attribute::<Tooltip>().unwrap();
        assert_eq!(&Tooltip::new("My awesome custom attribute!"), tooltip);
    }

    #[test]
    fn should_derive_custom_attributes_on_struct_fields() {
        #[derive(Reflect)]
        struct Slider {
            #[reflect(@0.0..=1.0)]
            #[reflect(@Tooltip::new("Range: 0.0 to 1.0"))]
            value: f32,
        }

        let TypeInfo::Struct(info) = Slider::type_info() else {
            panic!("expected struct info");
        };

        let field = info.field("value").unwrap();

        let range = field.get_attribute::<RangeInclusive<f64>>().unwrap();
        assert_eq!(&(0.0..=1.0), range);

        let tooltip = field.get_attribute::<Tooltip>().unwrap();
        assert_eq!(&Tooltip::new("Range: 0.0 to 1.0"), tooltip);
    }

    #[test]
    fn should_derive_custom_attributes_on_tuple_container() {
        #[derive(Reflect)]
        #[reflect(@Tooltip::new("My awesome custom attribute!"))]
        struct Slider(f32);

        let TypeInfo::TupleStruct(info) = Slider::type_info() else {
            panic!("expected tuple struct info");
        };

        let tooltip = info.get_attribute::<Tooltip>().unwrap();
        assert_eq!(&Tooltip::new("My awesome custom attribute!"), tooltip);
    }

    #[test]
    fn should_derive_custom_attributes_on_tuple_struct_fields() {
        #[derive(Reflect)]
        struct Slider(
            #[reflect(@0.0..=1.0)]
            #[reflect(@Tooltip::new("Range: 0.0 to 1.0"))]
            f32,
        );

        let TypeInfo::TupleStruct(info) = Slider::type_info() else {
            panic!("expected tuple struct info");
        };

        let field = info.field_at(0).unwrap();

        let range = field.get_attribute::<RangeInclusive<f64>>().unwrap();
        assert_eq!(&(0.0..=1.0), range);

        let tooltip = field.get_attribute::<Tooltip>().unwrap();
        assert_eq!(&Tooltip::new("Range: 0.0 to 1.0"), tooltip);
    }

    #[test]
    fn should_derive_custom_attributes_on_enum_container() {
        #[derive(Reflect)]
        #[reflect(@Tooltip::new("My awesome custom attribute!"))]
        enum Color {
            Transparent,
            Grayscale(f32),
            Rgb { r: u8, g: u8, b: u8 },
        }

        let TypeInfo::Enum(info) = Color::type_info() else {
            panic!("expected enum info");
        };

        let tooltip = info.get_attribute::<Tooltip>().unwrap();
        assert_eq!(&Tooltip::new("My awesome custom attribute!"), tooltip);
    }

    #[test]
    fn should_derive_custom_attributes_on_enum_variants() {
        #[derive(Reflect, Debug, PartialEq)]
        enum Display {
            Toggle,
            Slider,
            Picker,
        }

        #[derive(Reflect)]
        enum Color {
            #[reflect(@Display::Toggle)]
            Transparent,
            #[reflect(@Display::Slider)]
            Grayscale(f32),
            #[reflect(@Display::Picker)]
            Rgb { r: u8, g: u8, b: u8 },
        }

        let TypeInfo::Enum(info) = Color::type_info() else {
            panic!("expected enum info");
        };

        let VariantInfo::Unit(transparent_variant) = info.variant("Transparent").unwrap() else {
            panic!("expected unit variant");
        };

        let display = transparent_variant.get_attribute::<Display>().unwrap();
        assert_eq!(&Display::Toggle, display);

        let VariantInfo::Tuple(grayscale_variant) = info.variant("Grayscale").unwrap() else {
            panic!("expected tuple variant");
        };

        let display = grayscale_variant.get_attribute::<Display>().unwrap();
        assert_eq!(&Display::Slider, display);

        let VariantInfo::Struct(rgb_variant) = info.variant("Rgb").unwrap() else {
            panic!("expected struct variant");
        };

        let display = rgb_variant.get_attribute::<Display>().unwrap();
        assert_eq!(&Display::Picker, display);
    }

    #[test]
    fn should_derive_custom_attributes_on_enum_variant_fields() {
        #[derive(Reflect)]
        enum Color {
            Transparent,
            Grayscale(#[reflect(@0.0..=1.0_f32)] f32),
            Rgb {
                #[reflect(@0..=255u8)]
                r: u8,
                #[reflect(@0..=255u8)]
                g: u8,
                #[reflect(@0..=255u8)]
                b: u8,
            },
        }

        let TypeInfo::Enum(info) = Color::type_info() else {
            panic!("expected enum info");
        };

        let VariantInfo::Tuple(grayscale_variant) = info.variant("Grayscale").unwrap() else {
            panic!("expected tuple variant");
        };

        let field = grayscale_variant.field_at(0).unwrap();

        let range = field.get_attribute::<RangeInclusive<f32>>().unwrap();
        assert_eq!(&(0.0..=1.0), range);

        let VariantInfo::Struct(rgb_variant) = info.variant("Rgb").unwrap() else {
            panic!("expected struct variant");
        };

        let field = rgb_variant.field("g").unwrap();

        let range = field.get_attribute::<RangeInclusive<u8>>().unwrap();
        assert_eq!(&(0..=255), range);
    }

    #[test]
    fn should_allow_unit_struct_attribute_values() {
        #[derive(Reflect)]
        struct Required;

        #[derive(Reflect)]
        struct Foo {
            #[reflect(@Required)]
            value: i32,
        }

        let TypeInfo::Struct(info) = Foo::type_info() else {
            panic!("expected struct info");
        };

        let field = info.field("value").unwrap();
        assert!(field.has_attribute::<Required>());
    }

    #[test]
    fn should_accept_last_attribute() {
        #[derive(Reflect)]
        struct Foo {
            #[reflect(@false)]
            #[reflect(@true)]
            value: i32,
        }

        let TypeInfo::Struct(info) = Foo::type_info() else {
            panic!("expected struct info");
        };

        let field = info.field("value").unwrap();
        assert!(field.get_attribute::<bool>().unwrap());
    }
}
