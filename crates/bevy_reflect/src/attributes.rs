use crate::Reflect;
use bevy_utils::TypeIdMap;
use core::fmt::{Debug, Formatter};
use std::any::TypeId;

/// A collection of custom attributes for a type, field, or variant.
///
/// These attributes can be created with the [`Reflect` derive macro].
///
/// Attributes are stored by their [`TypeId`](std::any::TypeId).
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
#[derive(Default)]
pub struct CustomAttributes {
    attributes: TypeIdMap<CustomAttribute>,
}

impl CustomAttributes {
    /// Inserts a custom attribute into the collection.
    ///
    /// Note that this will overwrite any existing attribute of the same type.
    pub fn with_attribute<T: Reflect>(mut self, value: T) -> Self {
        self.attributes
            .insert(TypeId::of::<T>(), CustomAttribute::new(value));

        self
    }

    /// Returns `true` if this collection contains a custom attribute of the specified type.
    pub fn contains<T: Reflect>(&self) -> bool {
        self.attributes.contains_key(&TypeId::of::<T>())
    }

    /// Returns `true` if this collection contains a custom attribute with the specified [`TypeId`].
    pub fn contains_by_id(&self, id: TypeId) -> bool {
        self.attributes.contains_key(&id)
    }

    /// Gets a custom attribute by type.
    pub fn get<T: Reflect>(&self) -> Option<&T> {
        self.attributes.get(&TypeId::of::<T>())?.value::<T>()
    }

    /// Gets a custom attribute by its [`TypeId`].
    pub fn get_by_id(&self, id: TypeId) -> Option<&dyn Reflect> {
        Some(self.attributes.get(&id)?.reflect_value())
    }

    /// Returns an iterator over all custom attributes.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&TypeId, &dyn Reflect)> {
        self.attributes
            .iter()
            .map(|(key, value)| (key, value.reflect_value()))
    }

    /// Returns the number of custom attributes in this collection.
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Returns `true` if this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

impl Debug for CustomAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.attributes.values()).finish()
    }
}

struct CustomAttribute {
    value: Box<dyn Reflect>,
}

impl CustomAttribute {
    pub fn new<T: Reflect>(value: T) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    pub fn value<T: Reflect>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }

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
///
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

        /// Gets a custom attribute by its [`TypeId`](std::any::TypeId).
        ///
        /// This is the dynamic equivalent of [`get_attribute`](Self::get_attribute).
        pub fn get_attribute_by_id(&$self, id: ::std::any::TypeId) -> Option<&dyn $crate::Reflect> {
            $self.custom_attributes().get_by_id(id)
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a custom attribute of the specified type.")]
        #[doc = "\n\nFor dynamically checking if an attribute exists, see [`has_attribute_by_id`](Self::has_attribute_by_id)."]
        pub fn has_attribute<T: $crate::Reflect>(&$self) -> bool {
            $self.custom_attributes().contains::<T>()
        }

        #[doc = concat!("Returns `true` if this ", $term, " has a custom attribute with the specified [`TypeId`](::std::any::TypeId).")]
        #[doc = "\n\nThis is the dynamic equivalent of [`has_attribute`](Self::has_attribute)"]
        pub fn has_attribute_by_id(&$self, id: ::std::any::TypeId) -> bool {
            $self.custom_attributes().contains_by_id(id)
        }
    };
}

pub(crate) use impl_custom_attribute_methods;

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::type_info::Typed;
    use crate::{TypeInfo, VariantInfo};
    use std::ops::RangeInclusive;

    #[derive(Reflect, PartialEq, Debug)]
    struct Tooltip(String);

    impl Tooltip {
        fn new(value: impl Into<String>) -> Self {
            Self(value.into())
        }
    }

    #[test]
    fn should_get_custom_attribute() {
        let attributes = CustomAttributes::default().with_attribute(0.0..=1.0);

        let value = attributes.get::<RangeInclusive<f64>>().unwrap();
        assert_eq!(&(0.0..=1.0), value);
    }

    #[test]
    fn should_get_custom_attribute_dynamically() {
        let attributes = CustomAttributes::default().with_attribute(String::from("Hello, World!"));

        let value = attributes.get_by_id(TypeId::of::<String>()).unwrap();
        assert!(value
            .reflect_partial_eq(&String::from("Hello, World!"))
            .unwrap());
    }

    #[test]
    fn should_debug_custom_attributes() {
        let attributes = CustomAttributes::default().with_attribute("My awesome custom attribute!");

        let debug = format!("{:?}", attributes);

        assert_eq!(r#"{"My awesome custom attribute!"}"#, debug);

        #[derive(Reflect)]
        struct Foo {
            value: i32,
        }

        let attributes = CustomAttributes::default().with_attribute(Foo { value: 42 });

        let debug = format!("{:?}", attributes);

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
