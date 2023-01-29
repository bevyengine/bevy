//! By default, Bevy systems run in parallel with each other.
//! Unless the order is explicitly specified, their relative order is nondeterministic.
//!
//! In many cases, this doesn't matter and is in fact desirable!
//! Consider two systems, one which writes to resource A, and the other which writes to resource B.
//! By allowing their order to be arbitrary, we can evaluate them greedily, based on the data that is free.
//! Because their data accesses are **compatible**, there is no **observable** difference created based on the order they are run.
//!
//! But if instead we have two systems mutating the same data, or one reading it and the other mutating,
//! than the actual observed value will vary based on the nondeterministic order of evaluation.
//! These observable conflicts are called **system execution order ambiguities**.
//!
//! This example demonstrates how you might detect and resolve (or silence) these ambiguities.

use bevy::{ecs::schedule::ReportExecutionOrderAmbiguities, prelude::*};

fn main() {
    App::new()
        // This resource controls the reporting strategy for system execution order ambiguities
        .insert_resource(ReportExecutionOrderAmbiguities)
        .init_resource::<A>()
        .init_resource::<B>()
        // This pair of systems has an ambiguous order,
        // as their data access conflicts, and there's no order between them.
        .add_system(reads_a)
        .add_system(writes_a)
        // This trio of systems has conflicting data access,
        // but it's resolved with an explicit ordering
        .add_system(adds_one_to_b)
        .add_system(doubles_b.after(adds_one_to_b))
        // This system isn't ambiguous with adds_one_to_b,
        // due to the transitive ordering created.
        .add_system(reads_b.after(doubles_b))
        // This system will conflict with all of our writing systems
        // but we've silenced its ambiguity with adds_one_to_b.
        // This should only be done in the case of clear false positives:
        // leave a comment in your code justifying the decision!
        .add_system(reads_a_and_b.ambiguous_with(adds_one_to_b))
        // Be mindful, internal ambiguities are reported too!
        // If there are any ambiguities due solely to DefaultPlugins,
        // or between DefaultPlugins and any of your third party plugins,
        // please file a bug with the repo responsible!
        // Only *you* can prevent nondeterministic bugs due to greedy parallelism.
        .add_plugins(DefaultPlugins)
        // We're only going to run one frame of this app,
        // to make sure we can see the warnings at the start.
        .update();
}

#[derive(Resource, Debug, Default)]
struct A(usize);

#[derive(Resource, Debug, Default)]
struct B(usize);

// Data access is determined solely
fn reads_a(a: Res<A>) {
    dbg!(a);
}

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
    assert!((b.0 % 2 == 0) | (b.0 == usize::MAX));
    dbg!(b);
}

fn reads_a_and_b(a: Res<A>, b: Res<B>) {
    dbg!(a, b);
}
