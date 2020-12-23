use std::any::TypeId;

pub use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

/// You must manually register each instance of a generic type
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .register_type::<MyType<u32>>()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Reflect)]
struct MyType<T: Reflect> {
    value: T,
}

fn setup(type_registry: Res<TypeRegistry>) {
    let type_registry = type_registry.read();

    let registration = type_registry.get(TypeId::of::<MyType<u32>>()).unwrap();
    println!("Registration for {} exists", registration.short_name());

    // MyType<String> was not manually registered, so it does not exist
    assert!(type_registry.get(TypeId::of::<MyType<String>>()).is_none());
}
