use bevy::{
    component_registry::PropertyTypeRegistryContext,
    prelude::*,
    property::{ron::deserialize_dynamic_properties, AsProperties},
};
use serde::{Deserialize, Serialize};

fn main() {
    App::build()
        .add_default_plugins()
        // If you need to deserialize custom property types, register them like this:
        .register_property_type::<CustomProperty>()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Properties, Default)]
pub struct Test {
    a: usize,
    custom: CustomProperty,
    nested: Nested,
}

#[derive(Properties, Default)]
pub struct Nested {
    b: usize,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CustomProperty {
    a: usize,
}

impl Property for CustomProperty {
    fn any(&self) -> &dyn std::any::Any {
        self
    }
    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn clone_prop(&self) -> Box<dyn Property> {
        Box::new(self.clone())
    }
    fn set(&mut self, value: &dyn Property) {
        let value = value.any();
        if let Some(prop) = value.downcast_ref::<Self>() {
            *self = prop.clone();
        } else {
            panic!("prop value is not {}", std::any::type_name::<Self>());
        }
    }
    fn apply(&mut self, value: &dyn Property) {
        self.set(value);
    }
}

impl AsProperties for CustomProperty {
    fn as_properties(&self) -> Option<&dyn Properties> {
        None
    }
}

fn setup(property_type_registry: Res<PropertyTypeRegistryContext>) {
    let mut test = Test {
        a: 1,
        custom: CustomProperty { a: 10 },
        nested: Nested { b: 8 },
    };

    // You can set a property value like this. The type must match exactly or this will fail.
    test.set_prop_val::<usize>("a", 2);
    assert_eq!(test.a, 2);

    // You can also set properties dynamically. set_prop accepts any type that implements Property
    let x: u32 = 3;
    test.set_prop("a", &x);
    assert_eq!(test.a, 3);

    // DynamicProperties also implements the Properties trait.
    let mut patch = DynamicProperties::default();
    patch.set::<usize>("a", 4);

    // You can "apply" Properties on top of other Properties. This will only set properties with the same name and type.
    // You can use this to "patch" your components with new values.
    test.apply(&patch);
    assert_eq!(test.a, 4);

    // Properties implement the serde Serialize trait. You don't need to derive it yourself!

    let ron_string = serialize_ron(&test).unwrap();
    println!("{}\n", ron_string);

    // Dynamic properties can be deserialized
    let dynamic_properties =
        deserialize_dynamic_properties(&ron_string, &property_type_registry.value.read().unwrap())
            .unwrap();

    let round_tripped = serialize_ron(&dynamic_properties).unwrap();
    println!("{}", round_tripped);
    assert_eq!(ron_string, round_tripped);

    // This means you can patch Properties with dynamic properties deserialized from a string
    test.apply(&dynamic_properties);
}

fn serialize_ron<T>(properties: &T) -> Result<String, ron::Error>
where
    T: Serialize,
{
    let pretty_config = ron::ser::PrettyConfig::default().with_decimal_floats(true);
    let mut buf = Vec::new();
    let mut serializer = ron::ser::Serializer::new(&mut buf, Some(pretty_config), true)?;
    properties.serialize(&mut serializer)?;
    let ron_string = String::from_utf8(buf).unwrap();
    Ok(ron_string)
}
