use crate::{App, Plugin};
use bevy_ecs::{
    schedule::{IntoScheduleConfigs, ScheduleLabel, SystemSet},
    system::Local,
    world::World,
};

/// The schedule that contains the app logic that is evaluated each tick of [`App::update()`].
///
/// By default, it will run the following schedules in the given order:
///
/// On the first run of the schedule (and only on the first run), it will run:
/// * [`StateTransition`] [^1]
///      * This means that [`OnEnter(MyState::Foo)`] will be called *before* [`PreStartup`]
///        if `MyState` was added to the app with `MyState::Foo` as the initial state,
///        as well as [`OnEnter(MyComputedState)`] if it `compute`s to `Some(Self)` in `MyState::Foo`.
///      * If you want to run systems before any state transitions, regardless of which state is the starting state,
///        for example, for registering required components, you can just add a system
///        `.before(run_state_transition_schedule)` in [`StartupMain`].
/// * [`PreStartup`]
/// * [`Startup`]
/// * [`PostStartup`]
///
/// Then it will run:
/// * [`First`]
/// * [`PreUpdate`]
/// * `run_state_transition_schedule`, which runs the [`StateTransition`] [^1] schedule.
/// * [`RunFixedMainLoop`]
///     * This will run [`FixedMain`] zero to many times, based on how much time has elapsed.
/// * [`Update`]
/// * [`SpawnScene`]
/// * [`PostUpdate`]
/// * [`Last`]
///
/// # Rendering
///
/// Note rendering is not executed in the main schedule by default.
/// Instead, rendering is performed in a separate [`SubApp`]
/// which exchanges data with the main app in between the main schedule runs.
///
/// See [`RenderPlugin`] and [`PipelinedRenderingPlugin`] for more details.
///
/// [^1]: [`StateTransition`] is inserted only if you have `bevy_state` feature enabled. It is enabled in `default` features.
///
/// [`StateTransition`]: https://docs.rs/bevy/latest/bevy/prelude/struct.StateTransition.html
/// [`OnEnter(MyState::Foo)`]: https://docs.rs/bevy/latest/bevy/prelude/struct.OnEnter.html
/// [`OnEnter(MyComputedState)`]: https://docs.rs/bevy/latest/bevy/prelude/struct.OnEnter.html
/// [`RenderPlugin`]: https://docs.rs/bevy/latest/bevy/render/struct.RenderPlugin.html
/// [`PipelinedRenderingPlugin`]: https://docs.rs/bevy/latest/bevy/render/pipelined_rendering/struct.PipelinedRenderingPlugin.html
/// [`SubApp`]: crate::SubApp
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct EntryPoint;

/// TODO: Write Docs
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Main;

/// The schedule that runs once when the app starts.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct StartupMain;

/// The schedule that runs before [`Startup`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(StartupMain)]
pub struct PreStartup;

/// The schedule that runs once when the app starts.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(StartupMain)]
pub struct Startup;

/// The schedule that runs once after [`Startup`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(StartupMain)]
pub struct PostStartup;

/// Runs first in the schedule.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct First;

/// The schedule that contains logic that must run before [`Update`]. For example, a system that reads raw keyboard
/// input OS events into a `Messages` resource. This enables systems in [`Update`] to consume the messages from the `Messages`
/// resource without actually knowing about (or taking a direct scheduler dependency on) the "os-level keyboard event system".
///
/// [`PreUpdate`] exists to do "engine/plugin preparation work" that ensures the APIs consumed in [`Update`] are "ready".
/// [`PreUpdate`] abstracts out "pre work implementation details".
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct PreUpdate;

/// Runs the [`FixedMain`] schedule in a loop according until all relevant elapsed time has been "consumed".
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct RunFixedMainLoop;

/// Runs first in the [`FixedMain`] schedule.
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(FixedMain)]
pub struct FixedFirst;

/// The schedule that contains logic that must run before [`FixedUpdate`].
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(FixedMain)]
pub struct FixedPreUpdate;

/// The schedule that contains most gameplay logic, which runs at a fixed rate rather than every render frame.
/// For logic that should run once per render frame, use the [`Update`] schedule instead.
///
/// Examples of systems that should run at a fixed rate include (but are not limited to):
/// - Physics
/// - AI
/// - Networking
/// - Game rules
///
/// See the [`Update`] schedule for examples of systems that *should not* use this schedule.
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(FixedMain)]
pub struct FixedUpdate;

/// The schedule that runs after the [`FixedUpdate`] schedule, for reacting
/// to changes made in the main update logic.
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(FixedMain)]
pub struct FixedPostUpdate;

/// The schedule that runs last in [`FixedMain`]
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(FixedMain)]
pub struct FixedLast;

/// The schedule that contains systems which only run after a fixed period of time has elapsed.
///
/// This is run by the [`RunFixedMainLoop`] schedule. If you need to order your variable timestep systems
/// before or after the fixed update logic, add the systems to [`RunFixedMainLoop`] and then add
/// before/after the `run_fixed_main_schedule` system.
///
/// Frequency of execution is configured by inserting `Time<Fixed>` resource, 64 Hz by default.
/// See [this example](https://github.com/bevyengine/bevy/blob/latest/examples/time/time.rs).
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct FixedMain;

/// The schedule that contains any app logic that must run once per render frame.
/// For most gameplay logic, consider using [`FixedUpdate`] instead.
///
/// Examples of systems that should run once per render frame include (but are not limited to):
/// - UI
/// - Input handling
/// - Audio control
///
/// See the [`FixedUpdate`] schedule for examples of systems that *should not* use this schedule.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct Update;

/// The schedule that contains scene spawning.
///
/// This runs after [`Update`] and before [`PostUpdate`]. See the [`Main`] schedule for more details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SpawnScene;

/// The schedule that contains logic that must run after [`Update`]. For example, synchronizing "local transforms" in a hierarchy
/// to "global" absolute transforms. This enables the [`PostUpdate`] transform-sync system to react to "local transform" changes in
/// [`Update`] without the [`Update`] systems needing to know about (or add scheduler dependencies for) the "global transform sync system".
///
/// [`PostUpdate`] exists to do "engine/plugin response work" to things that happened in [`Update`].
/// [`PostUpdate`] abstracts out "implementation details" from users defining systems in [`Update`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct PostUpdate;

/// Runs last in the schedule.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[default_schedule(Main)]
pub struct Last;

/// Animation system set. This exists in [`PostUpdate`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct AnimationSystems;

/// Set enum for the systems relating to scene spawning.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SceneSpawnerSystems {
    /// Bevy's original scene system.
    WorldInstanceSpawn,
    /// Bevy's next-generation scene system
    SceneSpawn,
}

impl EntryPoint {
    /// A system that runs the "main schedule"
    pub fn run_main(world: &mut World, mut run_at_least_once: Local<bool>) {
        if !*run_at_least_once {
            world.run_schedule(StartupMain);
            *run_at_least_once = true;
        }

        world.run_schedule(Main);
    }
}

/// Initializes the [`Main`] schedule, sub schedules, and resources for a given [`App`].
pub struct MainSchedulePlugin;

impl Plugin for MainSchedulePlugin {
    fn build(&self, app: &mut App) {
        // simple "facilitator" schedules benefit from simpler single threaded scheduling
        app.init_schedule(StartupMain)
            .init_schedule(Main)
            .init_schedule(FixedMain)
            .configure_sets(StartupMain, (PreStartup, Startup, PostStartup).chain())
            .configure_sets(
                Main,
                (First, PreUpdate, RunFixedMainLoop, Update, PostUpdate, Last).chain(),
            )
            .configure_sets(Main, TransformGizmoRenderStep)
            .configure_sets(
                FixedMain,
                (
                    FixedFirst,
                    FixedPreUpdate,
                    FixedUpdate,
                    FixedPostUpdate,
                    FixedLast,
                )
                    .chain(),
            )
            .add_systems(EntryPoint, EntryPoint::run_main)
            .add_systems(
                Main,
                run_spawn_scene_schedule.after(Update).before(PostUpdate),
            );

        #[cfg(feature = "bevy_debug_stepping")]
        {
            use bevy_ecs::schedule::{IntoScheduleConfigs, Stepping};
            app.add_systems(
                EntryPoint,
                Stepping::begin_frame.before(EntryPoint::run_main),
            );
        }
    }
}

/// System to run the [`SpawnScene`] schedule.
pub fn run_spawn_scene_schedule(world: &mut World) {
    let _ = world.try_run_schedule(SpawnScene);
}

/// A System set that runs all systems needed to render the transform gizmo to the screen, used in the `bevy_gizmos_render` crate
/// This is defined here since it is used by many other
/// bevy crates to specify system orderings.
/// This is used for Bevy's Transform Gizmo and any other gizmo that render similar to it.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TransformGizmoRenderStep;
