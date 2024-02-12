use crate::Reflect;
use alloc::borrow::Cow;
use bevy_utils::HashMap;
use core::fmt::{Debug, Formatter};
use core::hash::Hash;

#[derive(Debug, Default)]
pub struct CustomAttributes {
    attributes: HashMap<Cow<'static, str>, CustomAttribute>,
}

impl CustomAttributes {
    pub fn with_attribute<T: Reflect>(
        mut self,
        name: impl Into<Cow<'static, str>>,
        value: T,
    ) -> Self {
        self.attributes
            .insert(name.into(), CustomAttribute::new(value));

        self
    }

    pub fn contains<K>(&self, name: &K) -> bool
    where
        Cow<'static, str>: core::borrow::Borrow<K>,
        K: Eq + Hash + ?Sized,
    {
        self.attributes.contains_key(name)
    }

    pub fn get<K>(&self, name: &K) -> Option<&CustomAttribute>
    where
        Cow<'static, str>: core::borrow::Borrow<K>,
        K: Eq + Hash + ?Sized,
    {
        self.attributes.get(name)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&str, &CustomAttribute)> {
        self.attributes.iter().map(|(k, v)| (k.as_ref(), v))
    }

    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

pub struct CustomAttribute {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::type_info::Typed;
    use crate::{TypeInfo, VariantInfo};

    #[test]
    fn should_create_custom_attributes() {
        let attributes = CustomAttributes::default()
            .with_attribute("min", 0.0_f32)
            .with_attribute("max", 1.0_f32);

        let value = attributes.get("max").unwrap().value::<f32>();

        assert_eq!(Some(&1.0), value);
    }

    #[test]
    fn should_debug_custom_attributes() {
        let attributes =
            CustomAttributes::default().with_attribute("label", "My awesome custom attribute!");

        let debug = format!("{:?}", attributes);

        assert_eq!(
            r#"CustomAttributes { attributes: {"label": "My awesome custom attribute!"} }"#,
            debug
        );

        #[derive(Reflect)]
        struct Foo {
            value: i32,
        }

        let attributes = CustomAttributes::default().with_attribute("foo", Foo { value: 42 });

        let debug = format!("{:?}", attributes);

        assert_eq!(
            r#"CustomAttributes { attributes: {"foo": bevy_reflect::attributes::tests::Foo { value: 42 }} }"#,
            debug
        );
    }

    #[test]
    fn should_derive_custom_attributes_on_struct_container() {
        #[derive(Reflect)]
        #[reflect(@::bevy_editor::hint = "My awesome custom attribute!")]
        struct Slider {
            value: f32,
        }

        let TypeInfo::Struct(info) = Slider::type_info() else {
            panic!("expected struct info");
        };

        let attributes = info.custom_attributes();

        let hint = attributes
            .get("::bevy_editor::hint")
            .unwrap()
            .value::<&str>();

        assert_eq!(Some(&"My awesome custom attribute!"), hint);
    }

    #[test]
    fn should_derive_custom_attributes_on_struct_fields() {
        #[derive(Reflect)]
        struct Slider {
            #[reflect(@min = 0.0, @max = 1.0)]
            #[reflect(@bevy_editor::hint = "Range: 0.0 to 1.0")]
            value: f32,
        }

        let TypeInfo::Struct(info) = Slider::type_info() else {
            panic!("expected struct info");
        };

        let field_attributes = info.field("value").unwrap().custom_attributes();

        let min = field_attributes.get("min").unwrap().value::<f64>();
        let max = field_attributes.get("max").unwrap().value::<f64>();
        let hint = field_attributes
            .get("bevy_editor::hint")
            .unwrap()
            .value::<&str>();

        assert_eq!(Some(&0.0), min);
        assert_eq!(Some(&1.0), max);
        assert_eq!(Some(&"Range: 0.0 to 1.0"), hint);
    }

    #[test]
    fn should_derive_custom_attributes_on_tuple_container() {
        #[derive(Reflect)]
        #[reflect(@::bevy_editor::hint = "My awesome custom attribute!")]
        struct Slider(f32);

        let TypeInfo::TupleStruct(info) = Slider::type_info() else {
            panic!("expected tuple struct info");
        };

        let attributes = info.custom_attributes();

        let hint = attributes
            .get("::bevy_editor::hint")
            .unwrap()
            .value::<&str>();

        assert_eq!(Some(&"My awesome custom attribute!"), hint);
    }

    #[test]
    fn should_derive_custom_attributes_on_tuple_struct_fields() {
        #[derive(Reflect)]
        struct Slider(
            #[reflect(@min = 0.0, @max = 1.0)]
            #[reflect(@bevy_editor::hint = "Range: 0.0 to 1.0")]
            f32,
        );

        let TypeInfo::TupleStruct(info) = Slider::type_info() else {
            panic!("expected tuple struct info");
        };

        let field_attributes = info.field_at(0).unwrap().custom_attributes();

        let min = field_attributes.get("min").unwrap().value::<f64>();
        let max = field_attributes.get("max").unwrap().value::<f64>();
        let hint = field_attributes
            .get("bevy_editor::hint")
            .unwrap()
            .value::<&str>();

        assert_eq!(Some(&0.0), min);
        assert_eq!(Some(&1.0), max);
        assert_eq!(Some(&"Range: 0.0 to 1.0"), hint);
    }

    #[test]
    fn should_derive_custom_attributes_on_enum_container() {
        #[derive(Reflect)]
        #[reflect(@::bevy_editor::hint = "My awesome custom attribute!")]
        enum Color {
            Transparent,
            Grayscale(f32),
            Rgb { r: u8, g: u8, b: u8 },
        }

        let TypeInfo::Enum(info) = Color::type_info() else {
            panic!("expected enum info");
        };

        let attributes = info.custom_attributes();

        let hint = attributes
            .get("::bevy_editor::hint")
            .unwrap()
            .value::<&str>();

        assert_eq!(Some(&"My awesome custom attribute!"), hint);
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
            #[reflect(@display = Display::Toggle)]
            Transparent,
            #[reflect(@display = Display::Slider)]
            Grayscale(f32),
            #[reflect(@display = Display::Picker)]
            Rgb { r: u8, g: u8, b: u8 },
        }

        let TypeInfo::Enum(info) = Color::type_info() else {
            panic!("expected enum info");
        };

        let VariantInfo::Unit(transparent_variant) = info.variant("Transparent").unwrap() else {
            panic!("expected unit variant");
        };

        let display = transparent_variant
            .custom_attributes()
            .get("display")
            .unwrap()
            .value::<Display>();
        assert_eq!(Some(&Display::Toggle), display);

        let VariantInfo::Tuple(grayscale_variant) = info.variant("Grayscale").unwrap() else {
            panic!("expected tuple variant");
        };

        let display = grayscale_variant
            .custom_attributes()
            .get("display")
            .unwrap()
            .value::<Display>();
        assert_eq!(Some(&Display::Slider), display);

        let VariantInfo::Struct(rgb_variant) = info.variant("Rgb").unwrap() else {
            panic!("expected struct variant");
        };

        let display = rgb_variant
            .custom_attributes()
            .get("display")
            .unwrap()
            .value::<Display>();
        assert_eq!(Some(&Display::Picker), display);
    }

    #[test]
    fn should_derive_custom_attributes_on_enum_variant_fields() {
        #[derive(Reflect)]
        enum Color {
            Transparent,
            Grayscale(#[reflect(@min = 0.0, @max = 1.0)] f32),
            Rgb {
                #[reflect(@min = 0u8, @max = 255u8)]
                r: u8,
                #[reflect(@min = 0u8, @max = 255u8)]
                g: u8,
                #[reflect(@min = 0u8, @max = 255u8)]
                b: u8,
            },
        }

        let TypeInfo::Enum(info) = Color::type_info() else {
            panic!("expected enum info");
        };

        let VariantInfo::Tuple(grayscale_variant) = info.variant("Grayscale").unwrap() else {
            panic!("expected tuple variant");
        };

        let grayscale_attributes = grayscale_variant.field_at(0).unwrap().custom_attributes();

        let min = grayscale_attributes.get("min").unwrap().value::<f64>();
        let max = grayscale_attributes.get("max").unwrap().value::<f64>();

        assert_eq!(Some(&0.0), min);
        assert_eq!(Some(&1.0), max);

        let VariantInfo::Struct(rgb_variant) = info.variant("Rgb").unwrap() else {
            panic!("expected struct variant");
        };

        let g_attributes = rgb_variant.field("g").unwrap().custom_attributes();

        let min = g_attributes.get("min").unwrap().value::<u8>();
        let max = g_attributes.get("max").unwrap().value::<u8>();

        assert_eq!(Some(&0), min);
        assert_eq!(Some(&255), max);
    }

    #[test]
    fn should_treat_path_as_bool_when_no_value_is_given() {
        #[derive(Reflect)]
        struct Foo {
            #[reflect(@bar)]
            value: i32,
        }

        let TypeInfo::Struct(info) = Foo::type_info() else {
            panic!("expected struct info");
        };

        let field_attributes = info.field("value").unwrap().custom_attributes();

        let bar = field_attributes.get("bar").unwrap().value::<bool>();

        assert_eq!(Some(&true), bar);
    }
}
