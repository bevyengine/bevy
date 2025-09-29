use crate::{App, Plugin};
use alloc::{vec, vec::Vec};
use bevy_ecs::{
    resource::Resource,
    schedule::{
        ExecutorKind, InternedScheduleLabel, IntoScheduleConfigs, Schedule, ScheduleLabel,
        SystemSet,
    },
    system::Local,
    world::{Mut, World},
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
///        for example, for registering required components, you can add your own custom startup schedule
///        before [`StateTransition`]. See [`MainScheduleOrder::insert_startup_before`] for more details.
/// * [`PreStartup`]
/// * [`Startup`]
/// * [`PostStartup`]
///
/// Then it will run:
/// * [`First`]
/// * [`PreUpdate`]
/// * [`StateTransition`] [^1]
/// * [`RunFixedMainLoop`]
///     * This will run [`FixedMain`] zero to many times, based on how much time has elapsed.
/// * [`Update`]
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
pub struct Main;

/// The schedule that runs before [`Startup`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PreStartup;

/// The schedule that runs once when the app starts.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Startup;

/// The schedule that runs once after [`Startup`].
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostStartup;

/// Runs first in the schedule.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct First;

/// The schedule that contains logic that must run before [`Update`]. For example, a system that reads raw keyboard
/// input OS events into a `Messages` resource. This enables systems in [`Update`] to consume the messages from the `Messages`
/// resource without actually knowing about (or taking a direct scheduler dependency on) the "os-level keyboard event system".
///
/// [`PreUpdate`] exists to do "engine/plugin preparation work" that ensures the APIs consumed in [`Update`] are "ready".
/// [`PreUpdate`] abstracts out "pre work implementation details".
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PreUpdate;

/// Runs the [`FixedMain`] schedule in a loop according until all relevant elapsed time has been "consumed".
///
/// If you need to order your variable timestep systems before or after
/// the fixed update logic, use the [`RunFixedMainLoopSystems`] system set.
///
/// Note that in contrast to most other Bevy schedules, systems added directly to
/// [`RunFixedMainLoop`] will *not* be parallelized between each other.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct RunFixedMainLoop;

/// Runs first in the [`FixedMain`] schedule.
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct FixedFirst;

/// The schedule that contains logic that must run before [`FixedUpdate`].
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
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
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct FixedUpdate;

/// The schedule that runs after the [`FixedUpdate`] schedule, for reacting
/// to changes made in the main update logic.
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct FixedPostUpdate;

/// The schedule that runs last in [`FixedMain`]
///
/// See the [`FixedMain`] schedule for details on how fixed updates work.
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct FixedLast;

/// The schedule that contains systems which only run after a fixed period of time has elapsed.
///
/// This is run by the [`RunFixedMainLoop`] schedule. If you need to order your variable timestep systems
/// before or after the fixed update logic, use the [`RunFixedMainLoopSystems`] system set.
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
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Update;

/// The schedule that contains scene spawning.
///
/// See the [`Main`] schedule for some details about how schedules are run.
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
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostUpdate;

/// Runs last in the schedule.
///
/// See the [`Main`] schedule for some details about how schedules are run.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Last;

/// Animation system set. This exists in [`PostUpdate`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct AnimationSystems;

/// Deprecated alias for [`AnimationSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `AnimationSystems`.")]
pub type Animation = AnimationSystems;

/// Defines the schedules to be run for the [`Main`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct MainScheduleOrder {
    /// The labels to run for the main phase of the [`Main`] schedule (in the order they will be run).
    pub labels: Vec<InternedScheduleLabel>,
    /// The labels to run for the startup phase of the [`Main`] schedule (in the order they will be run).
    pub startup_labels: Vec<InternedScheduleLabel>,
}

impl Default for MainScheduleOrder {
    fn default() -> Self {
        Self {
            labels: vec![
                First.intern(),
                PreUpdate.intern(),
                RunFixedMainLoop.intern(),
                Update.intern(),
                SpawnScene.intern(),
                PostUpdate.intern(),
                Last.intern(),
            ],
            startup_labels: vec![PreStartup.intern(), Startup.intern(), PostStartup.intern()],
        }
    }
}

impl MainScheduleOrder {
    /// Adds the given `schedule` after the `after` schedule in the main list of schedules.
    pub fn insert_after(&mut self, after: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .labels
            .iter()
            .position(|current| (**current).eq(&after))
            .unwrap_or_else(|| panic!("Expected {after:?} to exist"));
        self.labels.insert(index + 1, schedule.intern());
    }

    /// Adds the given `schedule` before the `before` schedule in the main list of schedules.
    pub fn insert_before(&mut self, before: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .labels
            .iter()
            .position(|current| (**current).eq(&before))
            .unwrap_or_else(|| panic!("Expected {before:?} to exist"));
        self.labels.insert(index, schedule.intern());
    }

    /// Adds the given `schedule` after the `after` schedule in the list of startup schedules.
    pub fn insert_startup_after(
        &mut self,
        after: impl ScheduleLabel,
        schedule: impl ScheduleLabel,
    ) {
        let index = self
            .startup_labels
            .iter()
            .position(|current| (**current).eq(&after))
            .unwrap_or_else(|| panic!("Expected {after:?} to exist"));
        self.startup_labels.insert(index + 1, schedule.intern());
    }

    /// Adds the given `schedule` before the `before` schedule in the list of startup schedules.
    pub fn insert_startup_before(
        &mut self,
        before: impl ScheduleLabel,
        schedule: impl ScheduleLabel,
    ) {
        let index = self
            .startup_labels
            .iter()
            .position(|current| (**current).eq(&before))
            .unwrap_or_else(|| panic!("Expected {before:?} to exist"));
        self.startup_labels.insert(index, schedule.intern());
    }
}

impl Main {
    /// A system that runs the "main schedule"
    pub fn run_main(world: &mut World, mut run_at_least_once: Local<bool>) {
        if !*run_at_least_once {
            world.resource_scope(|world, order: Mut<MainScheduleOrder>| {
                for &label in &order.startup_labels {
                    let _ = world.try_run_schedule(label);
                }
            });
            *run_at_least_once = true;
        }

        world.resource_scope(|world, order: Mut<MainScheduleOrder>| {
            for &label in &order.labels {
                let _ = world.try_run_schedule(label);
            }
        });
    }
}

/// Initializes the [`Main`] schedule, sub schedules, and resources for a given [`App`].
pub struct MainSchedulePlugin;

impl Plugin for MainSchedulePlugin {
    fn build(&self, app: &mut App) {
        // simple "facilitator" schedules benefit from simpler single threaded scheduling
        let mut main_schedule = Schedule::new(Main);
        main_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        let mut fixed_main_schedule = Schedule::new(FixedMain);
        fixed_main_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        let mut fixed_main_loop_schedule = Schedule::new(RunFixedMainLoop);
        fixed_main_loop_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        app.add_schedule(main_schedule)
            .add_schedule(fixed_main_schedule)
            .add_schedule(fixed_main_loop_schedule)
            .init_resource::<MainScheduleOrder>()
            .init_resource::<FixedMainScheduleOrder>()
            .add_systems(Main, Main::run_main)
            .add_systems(FixedMain, FixedMain::run_fixed_main)
            .configure_sets(
                RunFixedMainLoop,
                (
                    RunFixedMainLoopSystems::BeforeFixedMainLoop,
                    RunFixedMainLoopSystems::FixedMainLoop,
                    RunFixedMainLoopSystems::AfterFixedMainLoop,
                )
                    .chain(),
            );

        #[cfg(feature = "bevy_debug_stepping")]
        {
            use bevy_ecs::schedule::{IntoScheduleConfigs, Stepping};
            app.add_systems(Main, Stepping::begin_frame.before(Main::run_main));
        }
    }
}

/// Defines the schedules to be run for the [`FixedMain`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct FixedMainScheduleOrder {
    /// The labels to run for the [`FixedMain`] schedule (in the order they will be run).
    pub labels: Vec<InternedScheduleLabel>,
}

impl Default for FixedMainScheduleOrder {
    fn default() -> Self {
        Self {
            labels: vec![
                FixedFirst.intern(),
                FixedPreUpdate.intern(),
                FixedUpdate.intern(),
                FixedPostUpdate.intern(),
                FixedLast.intern(),
            ],
        }
    }
}

impl FixedMainScheduleOrder {
    /// Adds the given `schedule` after the `after` schedule
    pub fn insert_after(&mut self, after: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .labels
            .iter()
            .position(|current| (**current).eq(&after))
            .unwrap_or_else(|| panic!("Expected {after:?} to exist"));
        self.labels.insert(index + 1, schedule.intern());
    }

    /// Adds the given `schedule` before the `before` schedule
    pub fn insert_before(&mut self, before: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .labels
            .iter()
            .position(|current| (**current).eq(&before))
            .unwrap_or_else(|| panic!("Expected {before:?} to exist"));
        self.labels.insert(index, schedule.intern());
    }
}

impl FixedMain {
    /// A system that runs the fixed timestep's "main schedule"
    pub fn run_fixed_main(world: &mut World) {
        world.resource_scope(|world, order: Mut<FixedMainScheduleOrder>| {
            for &label in &order.labels {
                let _ = world.try_run_schedule(label);
            }
        });
    }
}

/// Set enum for the systems that want to run inside [`RunFixedMainLoop`],
/// but before or after the fixed update logic. Systems in this set
/// will run exactly once per frame, regardless of the number of fixed updates.
/// They will also run under a variable timestep.
///
/// This is useful for handling things that need to run every frame, but
/// also need to be read by the fixed update logic. See the individual variants
/// for examples of what kind of systems should be placed in each.
///
/// Note that in contrast to most other Bevy schedules, systems added directly to
/// [`RunFixedMainLoop`] will *not* be parallelized between each other.
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, SystemSet)]
pub enum RunFixedMainLoopSystems {
    /// Runs before the fixed update logic.
    ///
    /// A good example of a system that fits here
    /// is camera movement, which needs to be updated in a variable timestep,
    /// as you want the camera to move with as much precision and updates as
    /// the frame rate allows. A physics system that needs to read the camera
    /// position and orientation, however, should run in the fixed update logic,
    /// as it needs to be deterministic and run at a fixed rate for better stability.
    /// Note that we are not placing the camera movement system in `Update`, as that
    /// would mean that the physics system already ran at that point.
    ///
    /// # Example
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// App::new()
    ///   .add_systems(
    ///     RunFixedMainLoop,
    ///     update_camera_rotation.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop))
    ///   .add_systems(FixedUpdate, update_physics);
    ///
    /// # fn update_camera_rotation() {}
    /// # fn update_physics() {}
    /// ```
    BeforeFixedMainLoop,
    /// Contains the fixed update logic.
    /// Runs [`FixedMain`] zero or more times based on delta of
    /// [`Time<Virtual>`] and [`Time::overstep`].
    ///
    /// Don't place systems here, use [`FixedUpdate`] and friends instead.
    /// Use this system instead to order your systems to run specifically inbetween the fixed update logic and all
    /// other systems that run in [`RunFixedMainLoopSystems::BeforeFixedMainLoop`] or [`RunFixedMainLoopSystems::AfterFixedMainLoop`].
    ///
    /// [`Time<Virtual>`]: https://docs.rs/bevy/latest/bevy/prelude/struct.Virtual.html
    /// [`Time::overstep`]: https://docs.rs/bevy/latest/bevy/time/struct.Time.html#method.overstep
    /// # Example
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// App::new()
    ///   .add_systems(FixedUpdate, update_physics)
    ///   .add_systems(
    ///     RunFixedMainLoop,
    ///     (
    ///       // This system will be called before all interpolation systems
    ///       // that third-party plugins might add.
    ///       prepare_for_interpolation
    ///         .after(RunFixedMainLoopSystems::FixedMainLoop)
    ///         .before(RunFixedMainLoopSystems::AfterFixedMainLoop),
    ///     )
    ///   );
    ///
    /// # fn prepare_for_interpolation() {}
    /// # fn update_physics() {}
    /// ```
    FixedMainLoop,
    /// Runs after the fixed update logic.
    ///
    /// A good example of a system that fits here
    /// is a system that interpolates the transform of an entity between the last and current fixed update.
    /// See the [fixed timestep example] for more details.
    ///
    /// [fixed timestep example]: https://github.com/bevyengine/bevy/blob/main/examples/movement/physics_in_fixed_timestep.rs
    ///
    /// # Example
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// App::new()
    ///   .add_systems(FixedUpdate, update_physics)
    ///   .add_systems(
    ///     RunFixedMainLoop,
    ///     interpolate_transforms.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop));
    ///
    /// # fn interpolate_transforms() {}
    /// # fn update_physics() {}
    /// ```
    AfterFixedMainLoop,
}

/// Deprecated alias for [`RunFixedMainLoopSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `RunFixedMainLoopSystems`.")]
pub type RunFixedMainLoopSystem = RunFixedMainLoopSystems;
