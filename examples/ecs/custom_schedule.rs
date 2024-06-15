//! Demonstrates how to add custom schedules that run in Bevy's `Main` schedule, ordered relative to Bevy's built-in
//! schedules such as `Update` or `Last`.

use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;

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
    custom_update_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

    // Adding the schedule to the app does not automatically run the schedule. This merely registers the schedule so
    // that systems can look it up using the `Schedules` resource.
    app.add_schedule(custom_update_schedule);

    // Bevy `App`s have a `main_schedule_label` field that configures which schedule is run by the App's `runner`.
    // By default, this is `Main`. The `Main` schedule is responsible for running Bevy's main schedules such as
    // `Update`, `Startup` or `Last`.
    //
    // We can configure the `Main` schedule to run our custom update schedule relative to the existing ones by modifying
    // the `MainScheduleOrder` resource.
    //
    // Note that we modify `MainScheduleOrder` directly in `main` and not in a startup system. The reason for this is
    // that the `MainScheduleOrder` cannot be modified from systems that are run as part of the `Main` schedule.
    let mut main_schedule_order = app.world_mut().resource_mut::<MainScheduleOrder>();
    main_schedule_order.insert_after(Update, SingleThreadedUpdate);

    // Adding a custom startup schedule works similarly, but needs to use `insert_startup_after`
    // instead of `insert_after`.
    app.add_schedule(Schedule::new(CustomStartup));

    let mut main_schedule_order = app.world_mut().resource_mut::<MainScheduleOrder>();
    main_schedule_order.insert_startup_after(PreStartup, CustomStartup);

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
