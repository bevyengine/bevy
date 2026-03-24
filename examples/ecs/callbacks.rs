//! Sometimes, you want an extremely flexible way to store logic associated with an entity.
//! This example demonstrates how to store arbitrary systems in components and run them on demand.
//!
//! This pattern trades some performance for flexibility and works well for things like cutscenes, scripted events,
//! or one-off UI-driven interactions that don't need to run every frame.

use bevy::{ecs::system::SystemId, prelude::*};

fn main() {
    let mut app = App::new();
    app.add_systems(Startup, setup_callbacks);
    app.add_systems(Update, run_callbacks);
    app.run();
}

#[derive(Component)]
struct Callback {
    system_id: SystemId<(), ()>,
}

fn setup_callbacks(mut commands: Commands) {
    let trivial_callback = Callback {
        system_id: commands.register_system(|| {
            println!("This is the trivial callback system");
        }),
    };

    let ordinary_system_callback = Callback {
        system_id: commands.register_system(|query: Query<&Callback>| {
            let n_callbacks = query.iter().len();
            println!("This is the ordinary callback system. There are currently {n_callbacks} callbacks in the world.");
        }),
    };

    let exclusive_callback = Callback {
        system_id: commands.register_system(|world: &mut World| {
            let n_entities = world.entities().len();
            println!("This is the exclusive callback system. There are currently {n_entities} entities in the world.");
        }),
    };

    commands.spawn(trivial_callback);
    commands.spawn(ordinary_system_callback);
    commands.spawn(exclusive_callback);
}

// In many cases, you might want to use an observer to detect when a callback should run,
// triggering the callback in response to some entity-event!
fn run_callbacks(mut commands: Commands, query: Query<&Callback>) {
    for callback in query.iter() {
        commands.run_system(callback.system_id);
    }
}
