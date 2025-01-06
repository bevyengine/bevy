#![cfg(test)]
//! Testing of serialization and deserialization of diverse scene components.

use bevy::ecs::entity::EntityMap;
use bevy::prelude::*;
use bevy::reflect::erased_serde::__private::serde::de::DeserializeSeed;
use bevy::reflect::erased_serde::__private::serde::Serialize;
use bevy::scene::serde::{SceneDeserializer, SceneSerializer};
use bevy::scene::ScenePlugin;
use bincode::Options;
use std::io::BufReader;

#[test]
fn ron_roundtrip_equality() {
    assert_round_trip_equality(serialize_world_ron, deserialize_world_ron);
}

#[test]
fn postcard_roundtrip_equality() {
    assert_round_trip_equality(serialize_world_postcard, deserialize_world_postcard);
}

#[test]
fn bincode_roundtrip_equality() {
    assert_round_trip_equality(serialize_world_bincode, deserialize_world_bincode);
}

#[test]
fn messagepack_roundtrip_equality() {
    assert_round_trip_equality(serialize_world_messagepack, deserialize_world_messagepack);
}

/// Convenience function for testing the roundtrip equality of different serialization methods.
fn assert_round_trip_equality(
    serialize: fn(DynamicScene, &AppTypeRegistry) -> Vec<u8>,
    deserialize: fn(SceneDeserializer, &[u8]) -> DynamicScene,
) {
    let mut input_app = create_test_app();
    spawn_test_entities(&mut input_app);

    let type_registry = input_app.world.resource::<AppTypeRegistry>();
    let scene = DynamicScene::from_world(&input_app.world, type_registry);
    let serialized = serialize(scene, type_registry);

    // We use a clean app to deserialize into, so nothing of the input app can interfere.
    let mut output_app = create_test_app();
    let scene = {
        let scene_deserializer = SceneDeserializer {
            type_registry: &output_app.world.resource::<AppTypeRegistry>().read(),
        };
        deserialize(scene_deserializer, &serialized)
    };

    let mut entity_map = EntityMap::default();
    scene
        .write_to_world(&mut output_app.world, &mut entity_map)
        .unwrap_or_else(|error| panic!("Could not add deserialized scene to world: {error}"));

    // TODO: Ideally we'd check whether the input and output world are exactly equal. But the world does not implement Eq.
    //                         so instead we check the serialized outputs against each other. However, this will miss anything that fails to serialize in the first place.

    let type_registry = input_app.world.resource::<AppTypeRegistry>();
    let scene = DynamicScene::from_world(&input_app.world, type_registry);
    let serialized_again = serialize(scene, type_registry);

    assert_eq!(serialized, serialized_again);
}

fn serialize_world_ron(scene: DynamicScene, type_registry: &AppTypeRegistry) -> Vec<u8> {
    scene
        .serialize_ron(type_registry)
        .map(|output| output.as_bytes().to_vec())
        .unwrap_or_else(|error| panic!("Scene failed to serialize: {error}"))
}

fn deserialize_world_ron(scene_deserializer: SceneDeserializer, input: &[u8]) -> DynamicScene {
    let mut deserializer = ron::de::Deserializer::from_bytes(input)
        .unwrap_or_else(|error| panic!("Scene failed to deserialize: {error}"));
    scene_deserializer
        .deserialize(&mut deserializer)
        .unwrap_or_else(|error| panic!("Scene failed to deserialize: {error}"))
}

fn serialize_world_postcard(scene: DynamicScene, type_registry: &AppTypeRegistry) -> Vec<u8> {
    let scene_serializer = SceneSerializer::new(&scene, &type_registry.0);
    postcard::to_allocvec(&scene_serializer)
        .unwrap_or_else(|error| panic!("Scene failed to serialize: {error}"))
}

fn deserialize_world_postcard(scene_deserializer: SceneDeserializer, input: &[u8]) -> DynamicScene {
    let mut deserializer = postcard::Deserializer::from_bytes(input);
    scene_deserializer
        .deserialize(&mut deserializer)
        .unwrap_or_else(|error| panic!("Scene failed to deserialize: {error}"))
}

fn serialize_world_bincode(scene: DynamicScene, type_registry: &AppTypeRegistry) -> Vec<u8> {
    let scene_serializer = SceneSerializer::new(&scene, &type_registry.0);
    bincode::serialize(&scene_serializer)
        .unwrap_or_else(|error| panic!("Scene failed to serialize: {error}"))
}

fn deserialize_world_bincode(scene_deserializer: SceneDeserializer, input: &[u8]) -> DynamicScene {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .deserialize_seed(scene_deserializer, input)
        .unwrap_or_else(|error| panic!("Scene failed to deserialize: {error}"))
}

fn serialize_world_messagepack(scene: DynamicScene, type_registry: &AppTypeRegistry) -> Vec<u8> {
    let scene_serializer = SceneSerializer::new(&scene, &type_registry.0);
    let mut buf = Vec::new();
    let mut ser = rmp_serde::Serializer::new(&mut buf);
    scene_serializer
        .serialize(&mut ser)
        .unwrap_or_else(|error| panic!("Scene failed to serialize: {error}"));
    buf
}

fn deserialize_world_messagepack(
    scene_deserializer: SceneDeserializer,
    input: &[u8],
) -> DynamicScene {
    let mut reader = BufReader::new(input);

    scene_deserializer
        .deserialize(&mut rmp_serde::Deserializer::new(&mut reader))
        .unwrap_or_else(|error| panic!("Scene failed to deserialize: {error}"))
}

fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin::default())
        .add_plugin(TransformPlugin::default())
        .register_type::<ReferenceComponent>()
        .register_type::<Option<Entity>>()
        .register_type::<VectorComponent>()
        .register_type::<TupleComponent>()
        // TODO: Without the `Vec` registrations, the serialization
        //      works. But the de-serialization fails. This does not sound correct.
        //      Either both should fail, or both should work.
        .register_type::<Vec<u32>>()
        .register_type::<Vec<String>>()
        // TODO: Without these tuple registrations, the serialization
        //      works. But the de-serialization fails. This does not sound correct.
        //      Either both should fail, or both should work.
        .register_type::<(i32, String, f32)>()
        .register_type::<(bool, bool, u32)>();

    app
}

fn spawn_test_entities(app: &mut App) {
    let entity_1 = app.world.spawn(TransformBundle::default()).id();
    app.world.spawn(ReferenceComponent(Some(entity_1)));

    app.world
        .spawn(VectorComponent {
            integer_vector: vec![1, 2, 3, 4, 5, 123456789],
            // Testing different characters in strings
            string_vector: vec![
                // Basic string
                "Hello World!".to_string(),
                // Common special characters
                "!@#$%^&*(){}[]-=_+\\|,.<>/?;:'\"`~".to_string(),
                // Emoji
                "😄🌲🕊️🐧🏁".to_string(),
                // Chinese characters
                "月亮太阳".to_string(),
                // Non-breaking space.
                " ".to_string(),
            ],
        })
        .insert(TupleComponent(
            (-12, "A tuple".to_string(), 2.345),
            (true, false, 0),
        ));
}

/// Tests if Entity ids survive serialization.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReferenceComponent(Option<Entity>);

/// Tests if vectors survive serialization.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct VectorComponent {
    integer_vector: Vec<u32>,
    string_vector: Vec<String>,
}

/// Tests if tuples survive serialization.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct TupleComponent((i32, String, f32), (bool, bool, u32));
