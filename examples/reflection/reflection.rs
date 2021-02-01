use bevy::{
    prelude::*,
    reflect::{
        serde::{ReflectDeserializer, ReflectSerializer},
        DynamicStruct, TypeRegistry,
    },
};
use serde::de::DeserializeSeed;

/// This example illustrates how "reflection" works in Bevy. Reflection provide a way to dynamically interact with Rust types,
/// such as accessing fields by their string name. Reflection is a core part of Bevy and enables a number of interesting scenarios
/// (like scenes).
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .register_type::<Foo>()
        .register_type::<Bar>()
        .add_startup_system(setup.system())
        .run();
}

/// Deriving `Reflect` implements the relevant reflection traits. In this case, it implements the `Reflect` trait and the `Struct` trait
/// `derive(Reflect)` assumes that all fields also implement Reflect.
#[derive(Reflect)]
pub struct Foo {
    a: usize,
    nested: Bar,
    #[reflect(ignore)]
    _ignored: NonReflectedValue,
}

/// This `Bar` type is used in the `nested` field on the `Test` type. We must derive `Reflect` here too (or ignore it)
#[derive(Reflect)]
pub struct Bar {
    b: usize,
}

pub struct NonReflectedValue {
    _a: usize,
}

fn setup(type_registry: Res<TypeRegistry>) {
    let mut value = Foo {
        a: 1,
        _ignored: NonReflectedValue { _a: 10 },
        nested: Bar { b: 8 },
    };

    // You can set field values like this. The type must match exactly or this will fail.
    *value.get_field_mut("a").unwrap() = 2usize;
    assert_eq!(value.a, 2);
    assert_eq!(*value.get_field::<usize>("a").unwrap(), 2);

    // You can also get the &dyn Reflect value of a field like this
    let field = value.field("a").unwrap();

    // you can downcast Reflect values like this:
    assert_eq!(*field.downcast_ref::<usize>().unwrap(), 2);

    // DynamicStruct also implements the `Struct` and `Reflect` traits.
    let mut patch = DynamicStruct::default();
    patch.insert("a", 4usize);

    // You can "apply" Reflect implementations on top of other Reflect implementations.
    // This will only set fields with the same name, and it will fail if the types don't match.
    // You can use this to "patch" your types with new values.
    value.apply(&patch);
    assert_eq!(value.a, 4);

    let type_registry = type_registry.read();
    // By default, all derived `Reflect` types can be Serialized using serde. No need to derive Serialize!
    let serializer = ReflectSerializer::new(&value, &type_registry);
    let ron_string =
        ron::ser::to_string_pretty(&serializer, ron::ser::PrettyConfig::default()).unwrap();
    println!("{}\n", ron_string);

    // Dynamic properties can be deserialized
    let reflect_deserializer = ReflectDeserializer::new(&type_registry);
    let mut deserializer = ron::de::Deserializer::from_str(&ron_string).unwrap();
    let reflect_value = reflect_deserializer.deserialize(&mut deserializer).unwrap();

    // Deserializing returns a Box<dyn Reflect> value. Generally, deserializing a value will return the "dynamic" variant
    // of a type. For example, deserializing a struct will return the DynamicStruct type. "Value types" will be deserialized
    // as themselves.
    let _deserialized_struct = reflect_value.downcast_ref::<DynamicStruct>();

    // Reflect has its own `partial_eq` implementation, named `reflect_partial_eq`. This behaves like normal `partial_eq`, but it treats "dynamic" and
    // "non-dynamic" types the same. The `Foo` struct and deserialized `DynamicStruct` are considered equal for this reason:
    assert!(reflect_value.reflect_partial_eq(&value).unwrap());

    // By "patching" `Foo` with the deserialized DynamicStruct, we can "Deserialize" Foo.
    // This means we can serialize and deserialize with a single `Reflect` derive!
    value.apply(&*reflect_value);
}
