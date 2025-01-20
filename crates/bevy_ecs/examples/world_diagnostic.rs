//! In this example, we use a system to print diagnostic information about the world.
//!
//! This includes information about which components, bundles, and systems are registered,
//! as well as the order that systems will run in.

#![expect(
    missing_docs,
    reason = "Trivial example types do not require documentation."
)]

use bevy_ecs::prelude::*;
use bevy_ecs_macros::{ScheduleLabel, SystemSet};

fn empty_system() {}

fn first_system() {}

fn second_system() {}

fn increase_game_state_count(mut state: ResMut<GameState>) {
    state.counter += 1;
}

fn sync_counter(state: Res<GameState>, mut query: Query<&mut Counter>) {
    for mut counter in query.iter_mut() {
        counter.0 = state.counter;
    }
}

#[derive(Resource, Default)]
struct GameState {
    counter: usize,
}

#[derive(SystemSet, Hash, Clone, Copy, PartialEq, Eq, Debug)]
enum MySet {
    Set1,
    Set2,
}

#[derive(Component)]
struct Counter(usize);

#[derive(Component)]
struct Player;

#[derive(Component)]
#[component(storage = "SparseSet")]
struct HighlightFlag;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScheduleLabel {
    Foo,
    Bar,
}

/// A special label for diagnostic.
/// If a system has a commonly used label, like [`bevy_app::CoreSchedule`] it is not able to get
/// the corresponding [`Schedule`] instance and can't be inspected.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticLabel;

/// World diagnostic example.
fn diagnostic_world_system(world: &mut World) {
    println!("{}", world.diagnose_with_flattened().unwrap());
}

// If you do not have mutable access, you can also use [`World::diagnose`].
// This version will not include a flattened representation.
//
// fn diagnostic_world_system(world: &World) {
//     println!("{}", world.diagnose().unwrap());
// }

// In this example, we add a counter resource and increase its value in one system,
// while a different system prints debug information about the world.
fn main() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    {
        let mut diagnostic_schedule = Schedule::new(DiagnosticLabel);
        diagnostic_schedule.add_systems(diagnostic_world_system);
        world.add_schedule(diagnostic_schedule);
    }

    let mut schedule = Schedule::new(ScheduleLabel::Bar);
    schedule.configure_sets((MySet::Set1, MySet::Set2));

    schedule.add_systems(empty_system.in_set(MySet::Set1));
    schedule.add_systems(
        increase_game_state_count
            .in_set(MySet::Set1)
            .before(sync_counter),
    );
    schedule.add_systems(sync_counter);
    schedule.add_systems(first_system.before(second_system).in_set(MySet::Set2));
    schedule.add_systems(second_system.in_set(MySet::Set2));
    world.add_schedule(schedule);

    world.init_resource::<GameState>();

    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    let player = world.spawn(Player).id();
    // Create an archetype with one table component and one sparse set.
    world.spawn((Counter(1), HighlightFlag));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert(Counter(100));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert(HighlightFlag);
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).despawn();
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);
}
