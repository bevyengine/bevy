//! By default, Bevy systems run in parallel with each other.
//! Unless the order is explicitly specified, their relative order is nondeterministic.
//!
//! In many cases, this doesn't matter and is in fact desirable!
//! Consider two systems, one which writes to resource A, and the other which writes to resource B.
//! By allowing their order to be arbitrary, we can evaluate them greedily, based on the data that is free.
//! Because their data accesses are **compatible**, there is no **observable** difference created based on the order they are run.
//!
//! But if instead we have two systems mutating the same data, or one reading it and the other mutating,
//! then the actual observed value will vary based on the nondeterministic order of evaluation.
//! These observable conflicts are called **system execution order ambiguities**.
//!
//! This example demonstrates how you might detect and resolve (or silence) these ambiguities.

use bevy::{
    ecs::schedule::{LogLevel, ScheduleBuildSettings},
    prelude::*,
};

fn main() {
    App::new()
        // We can modify the reporting strategy for system execution order ambiguities on a per-schedule basis.
        // You must do this for each schedule you want to inspect; child schedules executed within an inspected
        // schedule do not inherit this modification.
        .edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
        .init_resource::<A>()
        .init_resource::<B>()
        .add_systems(
            Update,
            (
                // This pair of systems has an ambiguous order,
                // as their data access conflicts, and there's no order between them.
                reads_a,
                writes_a,
                // This pair of systems has conflicting data access,
                // but it's resolved with an explicit ordering:
                // the .after relationship here means that we will always double after adding.
                adds_one_to_b,
                doubles_b.after(adds_one_to_b),
                // This system isn't ambiguous with adds_one_to_b,
                // due to the transitive ordering created by our constraints:
                // if A is before B is before C, then A must be before C as well.
                reads_b.after(doubles_b),
                // This system will conflict with all of our writing systems
                // but we've silenced its ambiguity with adds_one_to_b.
                // This should only be done in the case of clear false positives:
                // leave a comment in your code justifying the decision!
                reads_a_and_b.ambiguous_with(adds_one_to_b),
            ),
        )
        // Be mindful, internal ambiguities are reported too!
        // If there are any ambiguities due solely to DefaultPlugins,
        // or between DefaultPlugins and any of your third party plugins,
        // please file a bug with the repo responsible!
        // Only *you* can prevent nondeterministic bugs due to greedy parallelism.
        .add_plugins(DefaultPlugins)
        .run();
}

#[derive(Resource, Debug, Default)]
struct A(usize);

#[derive(Resource, Debug, Default)]
struct B(usize);

// Data access is determined solely on the basis of the types of the system's parameters
// Every implementation of the `SystemParam` and `WorldQuery` traits must declare which data is used
// and whether or not it is mutably accessed.
fn reads_a(_a: Res<A>) {}

fn writes_a(mut a: ResMut<A>) {
    a.0 += 1;
}

fn adds_one_to_b(mut b: ResMut<B>) {
    b.0 = b.0.saturating_add(1);
}

fn doubles_b(mut b: ResMut<B>) {
    // This will overflow pretty rapidly otherwise
    b.0 = b.0.saturating_mul(2);
}

fn reads_b(b: Res<B>) {
    // This invariant is always true,
    // because we've fixed the system order so doubling always occurs after adding.
    assert!((b.0 % 2 == 0) || (b.0 == usize::MAX));
}

fn reads_a_and_b(a: Res<A>, b: Res<B>) {
    // Only display the first few steps to avoid burying the ambiguities in the console
    if b.0 < 10 {
        info!("{}, {}", a.0, b.0);
    }
}
