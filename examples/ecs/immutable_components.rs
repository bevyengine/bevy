//! This example demonstrates immutable components.

use bevy::prelude::*;

/// This component is mutable, the default case. This is indicated by components
/// implementing two traits, [`Component`], and [`ComponentMut`].
#[derive(Component)]
pub struct MyMutableComponent(bool);

/// This component is immutable. Once inserted into the ECS, it can only be viewed,
/// or removed. Replacement is also permitted, as this is equivalent to removal
/// and insertion.
///
/// Adding the `#[immutable]` attribute prevents the implementation of [`ComponentMut`]
/// in the derive macro.
#[derive(Component)]
#[immutable]
pub struct MyImmutableComponent(bool);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(world: &mut World) {
    // Immutable components can be inserted just like mutable components.
    let mut entity = world.spawn((MyMutableComponent(false), MyImmutableComponent(false)));

    // But where mutable components can be mutated...
    let mut my_mutable_component = entity.get_mut::<MyMutableComponent>().unwrap();
    my_mutable_component.0 = true;

    // ...immutable ones cannot. The below fails to compile as `MyImmutableComponent`
    // let mut my_immutable_component = entity.get_mut::<MyImmutableComponent>().unwrap();
    // my_immutable_component.0 = true;

    // Instead, you could take or replace the immutable component to update its value.
    let mut my_immutable_component = entity.take::<MyImmutableComponent>().unwrap();
    my_immutable_component.0 = true;
    entity.insert(my_immutable_component);
}
