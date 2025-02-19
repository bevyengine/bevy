//! Illustrates how to insert a `Reflect` `Component` into an `Entity` in Bevy.

use bevy::{
    // This is needed for the `insert_reflect()` method on `Commands`.
    ecs::reflect::ReflectCommandExt,
    log::LogPlugin,
    prelude::*,
    reflect::serde::ReflectDeserializer,
};
use serde::de::DeserializeSeed;

fn main() {
    let mut app = App::new();
    // Don't construct a window, but do print logs to the console.
    app.add_plugins((MinimalPlugins, LogPlugin::default()))
        // Bar will be automatically registered as it's a dependency of Foo
        .register_type::<Foo>()
        .add_systems(Startup, setup)
        .add_systems(Update, check_components);
    // Run a single update and just exit.
    app.update();
}

/// Deriving `Component` and `Reflect` here means we need to pass `Component` to the
/// `#[reflect(...)]` attribute.
///
/// The traits we pass to the `#[reflect(...)]` attribute are then passed through to the reflected
/// object's implementation of those traits. In this instance, we are telling the reflected object
/// that we want it to use our implementation of `Component`, and our implementation of `Debug`.
#[derive(Reflect, Component, Debug, Default, PartialEq, Eq)]
#[reflect(Component, Debug)]
pub struct Foo {
    a: usize,
    nested: Bar,

    /// All fields in a Reflected item need to be `Reflect`. In order to opt out of this, you add
    /// the `#[reflect(ignore)]` attribute. The `#[reflect(default = ...)]` attribute tells the
    /// compiler how to initialize this field when deserializing.
    #[reflect(ignore, default = "Default::default")]
    _ignored: NonReflectedValue,
}

/// This `Bar` type is used in the `nested` field of the `Foo` type. We must derive `Reflect` here
/// too (or ignore it).
#[derive(Reflect, Debug, Default, PartialEq, Eq)]
#[reflect(Debug)]
pub struct Bar {
    b: String,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct NonReflectedValue {
    _a: usize,
}

/// We'll spawn some Entities, and attach deserialized `Reflect` structs here.
fn setup(type_registry: Res<AppTypeRegistry>, mut commands: Commands) {
    let type_registry = type_registry.read();

    // Here are two different serialized versions of `Foo` objects. One serialized into JSON, and
    // the other into Ron
    let json_serialized = r#"{"insert_reflect::Foo":{"a":6,"nested":{"b":"from_json"}}}"#;
    let ron_serialized = r#"{"insert_reflect::Foo":(a:42,nested:(b:"from_ron"))}"#;

    // Spawn some entities to insert our components onto.
    let ron_entity = commands.spawn(Name::new("Ron Entity")).id();
    let json_entity = commands.spawn(Name::new("Json Entity")).id();

    // Construct the `ReflectDeserializer` from the `TypeRegistry`
    let reflect_deserializer = ReflectDeserializer::new(&type_registry);
    // Here is the JSON deserializer.
    let mut deserializer = serde_json::Deserializer::from_str(json_serialized);
    // And the `PartialReflect` gets constructed from the string here.
    let reflect_value = reflect_deserializer.deserialize(&mut deserializer).unwrap();

    info!("Inserting reflected component: {:?}", reflect_value);
    commands.entity(json_entity).insert_reflect(reflect_value);

    // `ReflectDeserializer::deserialize` takes and Owned `Self`, so we need to construct a new
    // `ReflectDeserializer` for each object we are constructing.
    let reflect_deserializer = ReflectDeserializer::new(&type_registry);
    // Here is the Ron deserializer.
    let mut deserializer = ron::de::Deserializer::from_str(ron_serialized).unwrap();
    // And, again, the `PartialReflect` gets constructed from the string here.
    let reflect_value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
    info!("Inserting reflected component: {:?}", reflect_value);
    commands.entity(ron_entity).insert_reflect(reflect_value);
}

/// Ensure that the `Component`s were correctly applied to the correct Entities.
fn check_components(foos: Query<(Entity, &Name, &Foo)>) {
    for (entity, name, component) in foos {
        info!("Found Entity@{entity:?} [{name}] with component: {component:?}");
        match name.to_string().as_ref() {
            "Json Entity" => {
                assert_eq!(
                    component,
                    &Foo {
                        a: 6,
                        nested: Bar {
                            b: "from_json".into()
                        },
                        _ignored: NonReflectedValue::default()
                    }
                );
            }
            "Ron Entity" => {
                assert_eq!(
                    component,
                    &Foo {
                        a: 42,
                        nested: Bar {
                            b: "from_ron".into()
                        },
                        _ignored: NonReflectedValue::default()
                    }
                );
            }
            _ => unreachable!(),
        }
    }
}
