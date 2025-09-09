//! Illustrates how "reflection" serialization works in Bevy.
//!
//! Deriving `Reflect` will also register `SerializationData`,
//! which powers reflect (de)serialization.
//! Serializing reflected data *does not* require deriving serde's
//! Serialize and Deserialize implementations.

use bevy::{
    prelude::*,
    reflect::serde::{ReflectDeserializer, ReflectSerializer},
};
use serde::de::DeserializeSeed;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (deserialize, serialize).chain())
        .run();
}

/// Deriving `Reflect` includes reflecting `SerializationData`
#[derive(Reflect)]
pub struct Player {
    name: String,
    health: u32,
}

const PLAYER_JSON: &str = r#"{
    "serialization::Player": {
        "name": "BevyPlayerOne",
        "health": 50
    }
}"#;

fn deserialize(type_registry: Res<AppTypeRegistry>) {
    let type_registry = type_registry.read();

    // a serde_json::Value that might have come from an API
    let value: serde_json::Value = serde_json::from_str(PLAYER_JSON).unwrap();

    // alternatively, `TypedReflectDeserializer` can be used if the type
    // is known.
    let deserializer = ReflectDeserializer::new(&type_registry);
    // deserialize
    let reflect_value = deserializer.deserialize(value).unwrap();
    // If Player implemented additional functionality, like Component,
    // this reflect_value could be used with commands.insert_reflect
    info!(?reflect_value);

    // `FromReflect` and `ReflectFromReflect` can yield a concrete value.
    let type_id = reflect_value.get_represented_type_info().unwrap().type_id();
    let reflect_from_reflect = type_registry
        .get_type_data::<ReflectFromReflect>(type_id)
        .unwrap();
    let player: Box<dyn Reflect> = reflect_from_reflect
        .from_reflect(reflect_value.as_partial_reflect())
        .unwrap();
    info!(?player);
}

fn serialize(type_registry: Res<AppTypeRegistry>) {
    let type_registry = type_registry.read();

    // a concrete value
    let value = Player {
        name: "BevyPlayerSerialize".to_string(),
        health: 80,
    };

    // By default, all derived `Reflect` types can be serialized using serde. No need to derive
    // Serialize!
    let serializer = ReflectSerializer::new(&value, &type_registry);
    let json = serde_json::to_string(&serializer).unwrap();
    info!(?json);
}
