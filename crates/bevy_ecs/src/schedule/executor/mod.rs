#[cfg(feature = "std")]
mod multi_threaded;
mod simple;
mod single_threaded;

use alloc::{vec, vec::Vec};
use bevy_utils::prelude::DebugName;
use core::any::TypeId;

#[expect(deprecated, reason = "We still need to support this.")]
pub use self::{simple::SimpleExecutor, single_threaded::SingleThreadedExecutor};

#[cfg(feature = "std")]
pub use self::multi_threaded::{MainThreadExecutor, MultiThreadedExecutor};

use fixedbitset::FixedBitSet;

use crate::{
    component::{CheckChangeTicks, Tick},
    error::{BevyError, ErrorContext, Result},
    prelude::{IntoSystemSet, SystemSet},
    query::FilteredAccessSet,
    schedule::{
        ConditionWithAccess, InternedSystemSet, SystemKey, SystemSetKey, SystemTypeSet,
        SystemWithAccess,
    },
    system::{RunSystemError, System, SystemIn, SystemParamValidationError, SystemStateFlags},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

/// Types that can run a [`SystemSchedule`] on a [`World`].
pub(super) trait SystemExecutor: Send + Sync {
    fn kind(&self) -> ExecutorKind;
    fn init(&mut self, schedule: &SystemSchedule);
    fn run(
        &mut self,
        schedule: &mut SystemSchedule,
        world: &mut World,
        skip_systems: Option<&FixedBitSet>,
        error_handler: fn(BevyError, ErrorContext),
    );
    fn set_apply_final_deferred(&mut self, value: bool);
}

/// Specifies how a [`Schedule`](super::Schedule) will be run.
///
/// The default depends on the target platform:
///  - [`SingleThreaded`](ExecutorKind::SingleThreaded) on Wasm.
///  - [`MultiThreaded`](ExecutorKind::MultiThreaded) everywhere else.
#[derive(PartialEq, Eq, Default, Debug, Copy, Clone)]
pub enum ExecutorKind {
    /// Runs the schedule using a single thread.
    ///
    /// Useful if you're dealing with a single-threaded environment, saving your threads for
    /// other things, or just trying minimize overhead.
    #[cfg_attr(any(target_arch = "wasm32", not(feature = "multi_threaded")), default)]
    SingleThreaded,
    /// Like [`SingleThreaded`](ExecutorKind::SingleThreaded) but calls [`apply_deferred`](crate::system::System::apply_deferred)
    /// immediately after running each system.
    #[deprecated(
        since = "0.17.0",
        note = "Use SingleThreaded instead. See https://github.com/bevyengine/bevy/issues/18453 for motivation."
    )]
    Simple,
    /// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
    #[cfg(feature = "std")]
    #[cfg_attr(all(not(target_arch = "wasm32"), feature = "multi_threaded"), default)]
    MultiThreaded,
}

/// Holds systems and conditions of a [`Schedule`](super::Schedule) sorted in topological order
/// (along with dependency information for `multi_threaded` execution).
///
/// Since the arrays are sorted in the same order, elements are referenced by their index.
/// [`FixedBitSet`] is used as a smaller, more efficient substitute of `HashSet<usize>`.
#[derive(Default)]
pub struct SystemSchedule {
    /// List of system node ids.
    pub(super) system_ids: Vec<SystemKey>,
    /// Indexed by system node id.
    pub(super) systems: Vec<SystemWithAccess>,
    /// Indexed by system node id.
    pub(super) system_conditions: Vec<Vec<ConditionWithAccess>>,
    /// Indexed by system node id.
    /// Number of systems that the system immediately depends on.
    #[cfg_attr(
        not(feature = "std"),
        expect(dead_code, reason = "currently only used with the std feature")
    )]
    pub(super) system_dependencies: Vec<usize>,
    /// Indexed by system node id.
    /// List of systems that immediately depend on the system.
    #[cfg_attr(
        not(feature = "std"),
        expect(dead_code, reason = "currently only used with the std feature")
    )]
    pub(super) system_dependents: Vec<Vec<usize>>,
    /// Indexed by system node id.
    /// List of sets containing the system that have conditions
    pub(super) sets_with_conditions_of_systems: Vec<FixedBitSet>,
    /// List of system set node ids.
    pub(super) set_ids: Vec<SystemSetKey>,
    /// Indexed by system set node id.
    pub(super) set_conditions: Vec<Vec<ConditionWithAccess>>,
    /// Indexed by system set node id.
    /// List of systems that are in sets that have conditions.
    ///
    /// If a set doesn't run because of its conditions, this is used to skip all systems in it.
    pub(super) systems_in_sets_with_conditions: Vec<FixedBitSet>,
}

impl SystemSchedule {
    /// Creates an empty [`SystemSchedule`].
    pub const fn new() -> Self {
        Self {
            systems: Vec::new(),
            system_conditions: Vec::new(),
            set_conditions: Vec::new(),
            system_ids: Vec::new(),
            set_ids: Vec::new(),
            system_dependencies: Vec::new(),
            system_dependents: Vec::new(),
            sets_with_conditions_of_systems: Vec::new(),
            systems_in_sets_with_conditions: Vec::new(),
        }
    }
}

/// A special [`System`] that instructs the executor to call
/// [`System::apply_deferred`] on the systems that have run but not applied
/// their [`Deferred`] system parameters (like [`Commands`]) or other system buffers.
///
/// ## Scheduling
///
/// `ApplyDeferred` systems are scheduled *by default*
/// - later in the same schedule run (for example, if a system with `Commands` param
///   is scheduled in `Update`, all the changes will be visible in `PostUpdate`)
/// - between systems with dependencies if the dependency [has deferred buffers]
///   (if system `bar` directly or indirectly depends on `foo`, and `foo` uses
///   `Commands` param, changes to the world in `foo` will be visible in `bar`)
///
/// ## Notes
/// - This system (currently) does nothing if it's called manually or wrapped
///   inside a [`PipeSystem`].
/// - Modifying a [`Schedule`] may change the order buffers are applied.
///
/// [`System::apply_deferred`]: crate::system::System::apply_deferred
/// [`Deferred`]: crate::system::Deferred
/// [`Commands`]: crate::prelude::Commands
/// [has deferred buffers]: crate::system::System::has_deferred
/// [`PipeSystem`]: crate::system::PipeSystem
/// [`Schedule`]: super::Schedule
#[doc(alias = "apply_system_buffers")]
pub struct ApplyDeferred;

/// Returns `true` if the [`System`] is an instance of [`ApplyDeferred`].
pub(super) fn is_apply_deferred(system: &dyn System<In = (), Out = ()>) -> bool {
    system.type_id() == TypeId::of::<ApplyDeferred>()
}

impl System for ApplyDeferred {
    type In = ();
    type Out = ();

    fn name(&self) -> DebugName {
        DebugName::borrowed("bevy_ecs::apply_deferred")
    }

    fn flags(&self) -> SystemStateFlags {
        // non-send , exclusive , no deferred
        SystemStateFlags::NON_SEND | SystemStateFlags::EXCLUSIVE
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        _world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        // This system does nothing on its own. The executor will apply deferred
        // commands from other systems instead of running this system.
        Ok(())
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {}

    fn run(
        &mut self,
        _input: SystemIn<'_, Self>,
        _world: &mut World,
    ) -> Result<Self::Out, RunSystemError> {
        // This system does nothing on its own. The executor will apply deferred
        // commands from other systems instead of running this system.
        Ok(())
    }

    fn apply_deferred(&mut self, _world: &mut World) {}

    fn queue_deferred(&mut self, _world: DeferredWorld) {}

    unsafe fn validate_param_unsafe(
        &mut self,
        _world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // This system is always valid to run because it doesn't do anything,
        // and only used as a marker for the executor.
        Ok(())
    }

    fn initialize(&mut self, _world: &mut World) -> FilteredAccessSet {
        FilteredAccessSet::new()
    }

    fn check_change_tick(&mut self, _check: CheckChangeTicks) {}

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        vec![SystemTypeSet::<Self>::new().intern()]
    }

    fn get_last_run(&self) -> Tick {
        // This system is never run, so it has no last run tick.
        Tick::MAX
    }

    fn set_last_run(&mut self, _last_run: Tick) {}
}

impl IntoSystemSet<()> for ApplyDeferred {
    type Set = SystemTypeSet<Self>;

    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::<Self>::new()
    }
}

/// These functions hide the bottom of the callstack from `RUST_BACKTRACE=1` (assuming the default panic handler is used).
///
/// The full callstack will still be visible with `RUST_BACKTRACE=full`.
/// They are specialized for `System::run` & co instead of being generic over closures because this avoids an
/// extra frame in the backtrace.
///
/// This is reliant on undocumented behavior in Rust's default panic handler, which checks the call stack for symbols
/// containing the string `__rust_begin_short_backtrace` in their mangled name.
mod __rust_begin_short_backtrace {
    use core::hint::black_box;

    #[cfg(feature = "std")]
    use crate::world::unsafe_world_cell::UnsafeWorldCell;
    use crate::{
        error::Result,
        system::{ReadOnlySystem, RunSystemError, ScheduleSystem},
        world::World,
    };

    /// # Safety
    /// See `System::run_unsafe`.
    // This is only used by `MultiThreadedExecutor`, and would be dead code without `std`.
    #[cfg(feature = "std")]
    #[inline(never)]
    pub(super) unsafe fn run_unsafe(
        system: &mut ScheduleSystem,
        world: UnsafeWorldCell,
    ) -> Result<(), RunSystemError> {
        let result = system.run_unsafe((), world);
        // Call `black_box` to prevent this frame from being tail-call optimized away
        black_box(());
        result
    }

    /// # Safety
    /// See `ReadOnlySystem::run_unsafe`.
    // This is only used by `MultiThreadedExecutor`, and would be dead code without `std`.
    #[cfg(feature = "std")]
    #[inline(never)]
    pub(super) unsafe fn readonly_run_unsafe<O: 'static>(
        system: &mut dyn ReadOnlySystem<In = (), Out = O>,
        world: UnsafeWorldCell,
    ) -> Result<O, RunSystemError> {
        // Call `black_box` to prevent this frame from being tail-call optimized away
        black_box(system.run_unsafe((), world))
    }

    #[inline(never)]
    pub(super) fn run(
        system: &mut ScheduleSystem,
        world: &mut World,
    ) -> Result<(), RunSystemError> {
        let result = system.run((), world);
        // Call `black_box` to prevent this frame from being tail-call optimized away
        black_box(());
        result
    }

    #[inline(never)]
    pub(super) fn run_without_applying_deferred(
        system: &mut ScheduleSystem,
        world: &mut World,
    ) -> Result<(), RunSystemError> {
        let result = system.run_without_applying_deferred((), world);
        // Call `black_box` to prevent this frame from being tail-call optimized away
        black_box(());
        result
    }

    #[inline(never)]
    pub(super) fn readonly_run<O: 'static>(
        system: &mut dyn ReadOnlySystem<In = (), Out = O>,
        world: &mut World,
    ) -> Result<O, RunSystemError> {
        // Call `black_box` to prevent this frame from being tail-call optimized away
        black_box(system.run((), world))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::{Component, In, IntoSystem, Resource, Schedule},
        schedule::ExecutorKind,
        system::{Populated, Res, ResMut, Single},
        world::World,
    };

    #[derive(Component)]
    struct TestComponent;

    const EXECUTORS: [ExecutorKind; 3] = [
        #[expect(deprecated, reason = "We still need to test this.")]
        ExecutorKind::Simple,
        ExecutorKind::SingleThreaded,
        ExecutorKind::MultiThreaded,
    ];

    #[derive(Resource, Default)]
    struct TestState {
        populated_ran: bool,
        single_ran: bool,
    }

    #[derive(Resource, Default)]
    struct Counter(u8);

    fn set_single_state(mut _single: Single<&TestComponent>, mut state: ResMut<TestState>) {
        state.single_ran = true;
    }

    fn set_populated_state(
        mut _populated: Populated<&TestComponent>,
        mut state: ResMut<TestState>,
    ) {
        state.populated_ran = true;
    }

    #[test]
    #[expect(clippy::print_stdout, reason = "std and println are allowed in tests")]
    fn single_and_populated_skipped_and_run() {
        for executor in EXECUTORS {
            std::println!("Testing executor: {executor:?}");

            let mut world = World::new();
            world.init_resource::<TestState>();

            let mut schedule = Schedule::default();
            schedule.set_executor_kind(executor);
            schedule.add_systems((set_single_state, set_populated_state));
            schedule.run(&mut world);

            let state = world.get_resource::<TestState>().unwrap();
            assert!(!state.single_ran);
            assert!(!state.populated_ran);

            world.spawn(TestComponent);

            schedule.run(&mut world);
            let state = world.get_resource::<TestState>().unwrap();
            assert!(state.single_ran);
            assert!(state.populated_ran);
        }
    }

    fn look_for_missing_resource(_res: Res<TestState>) {}

    #[test]
    #[should_panic]
    fn missing_resource_panics_simple() {
        let mut world = World::new();
        let mut schedule = Schedule::default();

        #[expect(deprecated, reason = "We still need to test this.")]
        schedule.set_executor_kind(ExecutorKind::Simple);
        schedule.add_systems(look_for_missing_resource);
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn missing_resource_panics_single_threaded() {
        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        schedule.add_systems(look_for_missing_resource);
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn missing_resource_panics_multi_threaded() {
        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.set_executor_kind(ExecutorKind::MultiThreaded);
        schedule.add_systems(look_for_missing_resource);
        schedule.run(&mut world);
    }

    #[test]
    fn piped_systems_first_system_skipped() {
        // This system should be skipped when run due to no matching entity
        fn pipe_out(_single: Single<&TestComponent>) -> u8 {
            42
        }

        fn pipe_in(_input: In<u8>, mut counter: ResMut<Counter>) {
            counter.0 += 1;
        }

        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);

        let counter = world.resource::<Counter>();
        assert_eq!(counter.0, 0);
    }

    #[test]
    fn piped_system_second_system_skipped() {
        // This system will be run before the second system is validated
        fn pipe_out(mut counter: ResMut<Counter>) -> u8 {
            counter.0 += 1;
            42
        }

        // This system should be skipped when run due to no matching entity
        fn pipe_in(_input: In<u8>, _single: Single<&TestComponent>, mut counter: ResMut<Counter>) {
            counter.0 += 1;
        }

        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);
        let counter = world.resource::<Counter>();
        assert_eq!(counter.0, 1);
    }

    #[test]
    #[should_panic]
    fn piped_system_first_system_panics() {
        // This system should panic when run because the resource is missing
        fn pipe_out(_res: Res<TestState>) -> u8 {
            42
        }

        fn pipe_in(_input: In<u8>) {}

        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn piped_system_second_system_panics() {
        fn pipe_out() -> u8 {
            42
        }

        // This system should panic when run because the resource is missing
        fn pipe_in(_input: In<u8>, _res: Res<TestState>) {}

        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);
    }

    // This test runs without panicking because we've
    // decided to use early-out behavior for piped systems
    #[test]
    fn piped_system_skip_and_panic() {
        // This system should be skipped when run due to no matching entity
        fn pipe_out(_single: Single<&TestComponent>) -> u8 {
            42
        }

        // This system should panic when run because the resource is missing
        fn pipe_in(_input: In<u8>, _res: Res<TestState>) {}

        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn piped_system_panic_and_skip() {
        // This system should panic when run because the resource is missing

        fn pipe_out(_res: Res<TestState>) -> u8 {
            42
        }

        // This system should be skipped when run due to no matching entity
        fn pipe_in(_input: In<u8>, _single: Single<&TestComponent>) {}

        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);
    }

    #[test]
    #[should_panic]
    fn piped_system_panic_and_panic() {
        // This system should panic when run because the resource is missing

        fn pipe_out(_res: Res<TestState>) -> u8 {
            42
        }

        // This system should panic when run because the resource is missing
        fn pipe_in(_input: In<u8>, _res: Res<TestState>) {}

        let mut world = World::new();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);
    }

    #[test]
    fn piped_system_skip_and_skip() {
        // This system should be skipped when run due to no matching entity

        fn pipe_out(_single: Single<&TestComponent>, mut counter: ResMut<Counter>) -> u8 {
            counter.0 += 1;
            42
        }

        // This system should be skipped when run due to no matching entity
        fn pipe_in(_input: In<u8>, _single: Single<&TestComponent>, mut counter: ResMut<Counter>) {
            counter.0 += 1;
        }

        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();

        schedule.add_systems(pipe_out.pipe(pipe_in));
        schedule.run(&mut world);

        let counter = world.resource::<Counter>();
        assert_eq!(counter.0, 0);
    }
}
