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

/// The `#[derive(Reflect)]` macro will automatically add any required bounds to `T`,
/// such as `Reflect` and `GetTypeRegistration`.
#[derive(Reflect)]
struct MyType<T> {
    value: T,
}

fn setup(type_registry: Res<AppTypeRegistry>) {
    let type_registry = type_registry.read();

    let registration = type_registry.get(TypeId::of::<MyType<u32>>()).unwrap();
    info!(
        "Registration for {} exists",
        registration.type_info().type_path(),
    );

    // MyType<String> was not manually registered, so it does not exist
    assert!(type_registry.get(TypeId::of::<MyType<String>>()).is_none());
}
