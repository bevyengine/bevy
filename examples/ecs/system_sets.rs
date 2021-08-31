use bevy::{app::AppExit, ecs::schedule::ShouldRun, prelude::*};

/// A [SystemLabel] can be applied as a label to systems and system sets,
/// which can then be referred to from other systems.
/// This is useful in case a user wants to e.g. run _before_ or _after_
/// some label.
/// `Clone`, `Hash`, `Debug`, `PartialEq`, `Eq`, are all required to derive
/// [SystemLabel].
#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
struct Physics;

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
struct PostPhysics;

/// Resource used to stop our example.
#[derive(Default)]
struct Done(bool);

/// This is used to show that within a [SystemSet], individual systems can also
/// be labelled, allowing further fine tuning of run ordering.
#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub enum PhysicsSystem {
    UpdateVelocity,
    Movement,
}

/// This example realizes the following scheme:
///
/// ```none
/// Physics                     (Criteria: App has run < 1.0 seconds)
///     \--> update_velocity        (via label PhysicsSystem::UpdateVelocity)
///     \--> movement               (via label PhysicsSystem::Movement)
/// PostPhysics                 (Criteria: Resource `done` is false)
///     \--> collision || sfx
/// Exit                        (Criteria: Resource `done` is true)
///     \--> exit
/// ```
///
/// The `Physics` label represents a [SystemSet] containing two systems.
/// This set's criteria is to stop after a second has elapsed.
/// The two systems (update_velocity, movement) runs in a specified order.
///
/// Another label `PostPhysics` uses run criteria to only run after `Physics` has finished.
/// This set's criteria is to run only when _not done_, as specified via a resource.
/// The two systems here (collision, sfx) are not specified to run in any order, and the actual
/// ordering can then change between invocations.
///
/// Lastly a system with run criterion _done_ is used to exit the app.
/// ```
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Done>()
        // Note that the system sets added in this example set their run criteria explicitly.
        // See the `ecs/state.rs` example for a pattern where run criteria are set implicitly for common
        // use cases- typically state transitions.
        // Also note that a system set has a single run criterion at most, which means using `.with_run_criteria(...)`
        // after `SystemSet::on_update(...)` would override the state transition criterion.
        .add_system_set(
            SystemSet::new()
                // This label is added to all systems in this set.
                // The label can then be referred to elsewhere (other sets).
                .label(Physics)
                // This criteria ensures this whole system set only runs when this system's
                // output says so (ShouldRun::Yes)
                .with_run_criteria(run_for_a_second)
                .with_system(
                    update_velocity
                        // Only applied to the `update_velocity` system
                        .label(PhysicsSystem::UpdateVelocity),
                )
                .with_system(
                    movement
                        // Only applied to the `movement` system
                        .label(PhysicsSystem::Movement)
                        // Enforce order within this system by specifying this
                        .after(PhysicsSystem::UpdateVelocity),
                ),
        )
        .add_system_set(
            SystemSet::new()
                .label(PostPhysics)
                // This whole set runs after `Physics` (which in this case is a label for
                // another set).
                // There is also `.before(..)`.
                .after(Physics)
                // This shows that we can modify existing run criteria results.
                // Here we create a _not done_ criteria by piping the output of
                // the `is_done` system and inverting the output.
                // Notice a string literal also works as a label.
                .with_run_criteria(RunCriteria::pipe("is_done_label", inverse.system()))
                // `collision` and `sfx` are not ordered with respect to
                // each other, and may run in any order
                .with_system(collision)
                .with_system(sfx),
        )
        .add_system(
            exit.after(PostPhysics)
                // Label the run criteria such that the `PostPhysics` set can reference it
                .with_run_criteria(is_done.label("is_done_label")),
        )
        .run();
}

/// Example of a run criteria.
/// Here we only want to run for a second, then stop.
fn run_for_a_second(time: Res<Time>, mut done: ResMut<Done>) -> ShouldRun {
    let elapsed = time.seconds_since_startup();
    if elapsed < 1.0 {
        info!(
            "We should run again. Elapsed/remaining: {:.2}s/{:.2}s",
            elapsed,
            1.0 - elapsed
        );
        ShouldRun::Yes
    } else {
        done.0 = true;
        ShouldRun::No
    }
}

/// Another run criteria, simply using a resource.
fn is_done(done: Res<Done>) -> ShouldRun {
    if done.0 {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

/// Used with [RunCritera::pipe], inverts the result of the
/// passed system.
fn inverse(input: In<ShouldRun>) -> ShouldRun {
    match input.0 {
        ShouldRun::No => ShouldRun::Yes,
        ShouldRun::Yes => ShouldRun::No,
        _ => unreachable!(),
    }
}

fn update_velocity() {
    info!("Updating velocity");
}

fn movement() {
    info!("Updating movement");
}

fn collision() {
    info!("Physics done- checking collisions");
}

fn sfx() {
    info!("Physics done- playing some sfx");
}

fn exit(mut app_exit_events: EventWriter<AppExit>) {
    info!("Exiting...");
    app_exit_events.send(AppExit);
}
