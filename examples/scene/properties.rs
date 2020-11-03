use bevy::{
    prelude::*,
    property::{ron::deserialize_dynamic_properties, PropertyTypeRegistry},
    scene::serialize_ron,
    type_registry::TypeRegistry,
};
use serde::{Deserialize, Serialize};

/// This example illustrates how Properties work. Properties provide a way to dynamically interact with Rust struct fields using
/// their names. Properties are a core part of Bevy and enable a number of interesting scenarios (like scenes). If you are
/// familiar with "reflection" in other languages, Properties are very similar to that concept.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // If you need to deserialize custom property types, register them like this:
        .register_property::<Test>()
        .register_property::<Nested>()
        .register_property::<CustomProperty>()
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

#[derive(Serialize, Deserialize, Default, Clone, Property)]
pub struct CustomProperty {
    a: usize,
}

fn setup(type_registry: Res<TypeRegistry>) {
    let mut test = Test {
        a: 1,
        custom: CustomProperty { a: 10 },
        nested: Nested { b: 8 },
    };

    // You can set a property value like this. The type must match exactly or this will fail.
    test.set_prop_val::<usize>("a", 2);
    assert_eq!(test.a, 2);
    assert_eq!(*test.prop_val::<usize>("a").unwrap(), 2);

    // You can also set properties dynamically. set_prop accepts any type that implements Property
    let x: u32 = 3;
    test.set_prop("a", &x);
    assert_eq!(test.a, 3);

    // DynamicProperties also implements the Properties trait.
    let mut patch = DynamicProperties::map();
    patch.set::<usize>("a", 4);

    // You can "apply" Properties on top of other Properties. This will only set properties with the same name and type.
    // You can use this to "patch" your properties with new values.
    test.apply(&patch);
    assert_eq!(test.a, 4);

    // All properties can be serialized.
    // If you #[derive(Properties)] your type doesn't even need to directly implement the Serde trait!
    let registry = type_registry.property.read();
    let ron_string = serialize_property(&test, &registry);
    println!("{}\n", ron_string);

    // Dynamic properties can be deserialized
    let dynamic_properties = deserialize_dynamic_properties(&ron_string, &registry).unwrap();

    let round_tripped = serialize_property(&dynamic_properties, &registry);
    println!("{}", round_tripped);
    assert_eq!(ron_string, round_tripped);

    // This means you can patch Properties with dynamic properties deserialized from a string
    test.apply(&dynamic_properties);

    // Properties can also be sequences.
    // Sequences from std::collections (Vec, VecDeque) already implement the Properties trait
    let mut seq = vec![1u32, 2u32];
    let mut patch = DynamicProperties::seq();
    patch.push(Box::new(3u32), None);
    seq.apply(&patch);
    assert_eq!(seq[0], 3);

    let ron_string = serialize_property(&patch, &registry);
    println!("{}\n", ron_string);
    let dynamic_properties = deserialize_dynamic_properties(&ron_string, &registry).unwrap();
    let round_tripped = serialize_property(&dynamic_properties, &registry);
    assert_eq!(ron_string, round_tripped);
}

fn serialize_property<T>(property: &T, registry: &PropertyTypeRegistry) -> String
where
    T: Property,
{
    serialize_ron(property.serializable(registry).borrow()).unwrap()
}
