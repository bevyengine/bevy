//! This example demonstrates how you could create your own custom subapps to run tasks based off of the main app.

use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

#[derive(Clone, Copy, AppLabel, Hash, PartialEq, Eq, Debug, Default)]
struct OurCustomSubApp;

/// A ref to which main world entity our subapp entity refers to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Component)]
struct MainWorldEntity(pub Entity);

fn extract(main_world: &mut World, our_world: &mut World) {
    let mut query_state_existing_entities = our_world.query::<&MainWorldEntity>();
    let existing_entities = query_state_existing_entities
        .iter(our_world)
        .map(|x| x.0)
        .collect::<Vec<_>>();
    // For each currently non existing entity in main world, create a variant for our own world
    let mut query_state = main_world.query::<(Entity, &Name)>();
    for (e, name) in query_state.iter(main_world) {
        if !existing_entities.contains(&e) {
            our_world.spawn((MainWorldEntity(e), name.clone()));
        }
    }
}

fn exists_hi_system(query: Populated<&Name, With<MainWorldEntity>>, mut found: Local<bool>) {
    if *found {
        return;
    }
    for i in query.iter() {
        if i.as_str() == "hello there" {
            *found = true;
            println!("{i}");
        }
    }
}

fn main() {
    let mut sub_app = SubApp::new();
    sub_app.set_extract(extract);
    // Sets the default schedule to run whenever sub_app.update() gets called.
    sub_app.update_schedule = Some(Main.intern());
    sub_app.add_systems(Main, exists_hi_system);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_sub_app(OurCustomSubApp, sub_app);
    app.world_mut().spawn(Name::new("hello there"));
    app.run();
}
