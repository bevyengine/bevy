use bevy::prelude::*;
use rand::Rng;

// This example illustrates how to react to component change
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(change_component.system())
        .add_system(change_detection.system())
        .add_system(flags_monitoring.system())
        .run();
}

#[derive(Debug)]
struct MyComponent(f64);

fn setup(commands: &mut Commands) {
    commands.spawn((MyComponent(0.),));
    commands.spawn((Transform::default(),));
}

fn change_component(time: Res<Time>, mut query: Query<(Entity, &mut MyComponent)>) {
    for (entity, mut component) in query.iter_mut() {
        if rand::thread_rng().gen_bool(0.1) {
            info!("changing component {:?}", entity);
            component.0 = time.seconds_since_startup();
        }
    }
}

// There are query filters for `Changed<T>`, `Added<T>` and `Mutated<T>`
// Only entities matching the filters will be in the query
fn change_detection(query: Query<(Entity, &MyComponent), Changed<MyComponent>>) {
    for (entity, component) in query.iter() {
        info!("{:?} changed: {:?}", entity, component,);
    }
}

// By looking at flags, the query is not filtered but the information is available
fn flags_monitoring(query: Query<(Entity, Option<&MyComponent>, Flags<MyComponent>)>) {
    for (entity, component, flags) in query.iter() {
        info!(
            "{:?}: {:?} -> with: {:?} - added: {:?} - mutated: {:?} - changed: {:?}",
            entity,
            component,
            flags.with(),
            flags.added(),
            flags.mutated(),
            flags.changed()
        );
    }
}
