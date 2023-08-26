//! Demonstrates how reflection is used with generic Rust types.

use bevy::prelude::*;
use std::any::TypeId;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // You must manually register each instance of a generic type
        .register_type::<MyType<u32>>()
        .add_systems(Startup, setup)
        .run();
}

#[derive(Reflect)]
struct MyType<T: Reflect> {
    value: T,
}

fn setup(type_registry: Res<AppTypeRegistry>) {
    let type_registry = type_registry.read();

    let registration = type_registry.get(TypeId::of::<MyType<u32>>()).unwrap();
    info!("Registration for {} exists", registration.short_name());

    // MyType<String> was not manually registered, so it does not exist
    assert!(type_registry.get(TypeId::of::<MyType<String>>()).is_none());
}
