//! In this example, we use a system to print diagnostic information about the world.

#![expect(missing_docs)]

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

// Todo Double check. Base sets seem to have been removed as of V0.11.
// #[derive(SystemSet, Hash, Clone, Copy, PartialEq, Eq, Debug)]
// enum MyBaseSet {
//     BaseSet1,
//     BaseSet2,
// }

#[derive(Component)]
struct Counter(usize);

#[expect(dead_code)]
#[derive(Component)]
struct HitPoint(usize);

#[derive(Component)]
#[component(storage = "SparseSet")]
struct HighlightFlag;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScheduleLabel {
    Foo,
    Bar,
}

/// A special label for diagnostic.
/// If the diagnostic system running in a commonly used label, like the ones
/// defined as `bevy_app::CoreSchedule`, it is not able to get the corresponding
/// [`Schedule`] instance. ([`World::run_schedule_ref`] for reference)
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiagnosticLabel;

/// World diagnostic example.
/// Can be called directly on a World or run as a system.
fn diagnostic_world_system(world: &World) {
    println!("{}", world.diagnose().unwrap());
}

// In this example, we add a counter resource and increase its value in one system,
// while a different system prints the current count to the console.
fn main() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    {
        let mut diagnostic_schedule = Schedule::new(DiagnosticLabel);
        diagnostic_schedule.add_systems(diagnostic_world_system);
        world.add_schedule(diagnostic_schedule);
    }

    let mut schedule = Schedule::new(ScheduleLabel::Bar);
    schedule.configure_sets(MySet::Set1);
    schedule.configure_sets(MySet::Set2);

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

    let player = world.spawn(HitPoint(100)).id();
    // create an archetype with 2 table components and 1 sparse set
    world.spawn((HitPoint(100), Counter(1), HighlightFlag));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert((Counter(0),));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).insert((HighlightFlag,));
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);

    world.entity_mut(player).despawn();
    world.run_schedule(ScheduleLabel::Bar);
    world.run_schedule(DiagnosticLabel);
}
