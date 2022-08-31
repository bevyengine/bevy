//! Demonstrates how reflection is used with generic Rust types.

use bevy::prelude::*;
use std::any::TypeId;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // You must manually register each instance of a generic type
        .register_type::<MyType<u32>>()
        .add_startup_system(setup)
        .run();
}

/// Generic arguments _must_ be bound certain reflection traits, namely [`Reflect`].
///
/// Until the reflection API stabilizes, these trait bounds are liable to change. While we currently
/// only require [`Reflect`] right now, we may eventually require others.
///
/// Rather than adding each trait to our generic arguments manually (and having to update them
/// if the requirements ever change), we can simply use the catch-all trait, [`Reflectable`].
#[derive(Reflect)]
struct MyType<T: Reflectable> {
    value: T,
}

fn setup(type_registry: Res<AppTypeRegistry>) {
    let type_registry = type_registry.read();

    let registration = type_registry.get(TypeId::of::<MyType<u32>>()).unwrap();
    info!("Registration for {} exists", registration.short_name());

    // MyType<String> was not manually registered, so it does not exist
    assert!(type_registry.get(TypeId::of::<MyType<String>>()).is_none());
}
