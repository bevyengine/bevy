//! This example illustrates how to react to component change.

use bevy::prelude::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(change_component)
        .add_system(change_detection)
        .add_system(tracker_monitoring)
        .run();
}

#[derive(Component, Debug)]
struct MyComponent(f64);

fn setup(mut commands: Commands) {
    commands.spawn().insert(MyComponent(0.));
    commands.spawn().insert(Transform::identity());
}

fn change_component(time: Res<Time>, mut query: Query<(Entity, &mut MyComponent)>) {
    for (entity, mut component) in &mut query {
        if rand::thread_rng().gen_bool(0.1) {
            info!("changing component {:?}", entity);
            component.0 = time.seconds_since_startup();
        }
    }
}

// There are query filters for `Changed<T>` and `Added<T>`
// Only entities matching the filters will be in the query
fn change_detection(query: Query<(Entity, &MyComponent), Changed<MyComponent>>) {
    for (entity, component) in &query {
        info!("{:?} changed: {:?}", entity, component,);
    }
}

// By looking at trackers, the query is not filtered but the information is available
fn tracker_monitoring(
    query: Query<(
        Entity,
        Option<&MyComponent>,
        Option<ChangeTrackers<MyComponent>>,
    )>,
) {
    for (entity, component, trackers) in &query {
        info!("{:?}: {:?} -> {:?}", entity, component, trackers);
    }
}
