//! Demonstrates how to add custom schedules that run in Bevy's `Main` schedule, ordered relative to Bevy's built-in
//! system sets such as `Update` or `Last`.

use bevy::{
    app::StartupMain,
    ecs::schedule::{ScheduleLabel, SingleThreadedExecutor},
    prelude::*,
};

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct SingleThreadedUpdate;

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct CustomStartup;

fn main() {
    let mut app = App::new();

    // Create a new [`Schedule`]. For demonstration purposes, we configure it to use a single threaded executor so that
    // systems in this schedule are never run in parallel. However, this is not a requirement for custom schedules in
    // general.
    let mut custom_update_schedule = Schedule::new(SingleThreadedUpdate);
    custom_update_schedule.set_executor(SingleThreadedExecutor::new());

    // Adding the schedule to the app does not automatically run the schedule. This merely registers the schedule so
    // that systems can look it up using the `Schedules` resource.
    app.add_schedule(custom_update_schedule);

    // Bevy `App`s have a `main_schedule_label` field that configures which schedule is run by the App's `runner`.
    // By default, this is `Main`. The `Main` schedule runs the bulk of Bevy's systems.
    //
    // We can create an exclusive system to run our custom schedule and use system ordering to place
    // that schedule where we need it to run.
    fn run_single_threaded_update_schedule(world: &mut World) {
        world.run_schedule(SingleThreadedUpdate);
    }
    app.add_systems(
        Main,
        run_single_threaded_update_schedule
            .after(Update)
            .before(PostUpdate),
    );

    // Adding a custom startup schedule works similarly, but with `StartupMain`.
    app.add_schedule(Schedule::new(CustomStartup));
    fn run_custom_startup(world: &mut World) {
        world.run_schedule(CustomStartup);
    }
    app.add_systems(StartupMain, run_custom_startup.before(PreStartup));

    app.add_systems(SingleThreadedUpdate, single_threaded_update_system)
        .add_systems(CustomStartup, custom_startup_system)
        .add_systems(PreStartup, pre_startup_system)
        .add_systems(Startup, startup_system)
        .add_systems(First, first_system)
        .add_systems(Update, update_system)
        .add_systems(Last, last_system)
        .run();
}

fn pre_startup_system() {
    println!("Pre Startup");
}

fn startup_system() {
    println!("Startup");
}

fn custom_startup_system() {
    println!("Custom Startup");
}

fn first_system() {
    println!("First");
}

fn update_system() {
    println!("Update");
}

fn single_threaded_update_system() {
    println!("Single Threaded Update");
}

fn last_system() {
    println!("Last");
}
