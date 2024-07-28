//! This example illustrates how to react to component change.

use bevy::prelude::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (change_component, change_component_2, change_detection),
        )
        .run();
}

#[derive(Component, PartialEq, Debug)]
struct MyComponent(f32);

fn setup(mut commands: Commands) {
    // Note the first change detection log correctly points to this line because the component is
    // added. Although commands are deferred, they are able to track the original calling location.
    commands.spawn(MyComponent(0.));
    commands.spawn(Transform::IDENTITY);
}

fn change_component(time: Res<Time>, mut query: Query<(Entity, &mut MyComponent)>) {
    for (entity, mut component) in &mut query {
        if rand::thread_rng().gen_bool(0.1) {
            let new_component = MyComponent(time.elapsed_seconds().round());
            info!("{entity:?}: New value: {new_component:?}");
            // Change detection occurs on mutable dereference, and does not consider whether or not
            // a value is actually equal. To avoid triggering change detection when nothing has
            // actually changed, you can use the `set_if_neq` method on any component or resource
            // that implements PartialEq.
            component.set_if_neq(new_component);
        }
    }
}

/// This is a duplicate of the `change_component` system, added to show that change tracking can
/// help you find *where* your component is being changed, when there are multiple possible
/// locations.
fn change_component_2(time: Res<Time>, mut query: Query<(Entity, &mut MyComponent)>) {
    for (entity, mut component) in &mut query {
        if rand::thread_rng().gen_bool(0.1) {
            let new_component = MyComponent(time.elapsed_seconds().round());
            info!("{entity:?}: New value: {new_component:?}");
            component.set_if_neq(new_component);
        }
    }
}

/// Query filters like [`Changed<T>`] and [`Added<T>`] ensure only entities matching these filters
/// will be returned by the query.
///
/// Using the [`Ref<T>`] system param allows you to access change detection information, but does
/// not filter the query.
fn change_detection(query: Query<Ref<MyComponent>, Changed<MyComponent>>) {
    for component in &query {
        // By default, you can only tell that a component was changed.
        //
        // This is useful, but what if you have multiple systems modifying the same component, how
        // will you know which system is causing the component to change?
        warn!(
            "Change detected!\n\t-> value: {:?}\n\t-> added: {}\n\t-> changed: {}\n\t-> changed by: {}",
            component,
            component.is_added(),
            component.is_changed(),
            // If you enable the `track_change_detection` feature, you can unlock the `changed_by()`
            // method. It returns the the file and line number that the component or resource was
            // changed in. It's not recommended for released games, but great for debugging!
            component.changed_by()
        );
    }
}
