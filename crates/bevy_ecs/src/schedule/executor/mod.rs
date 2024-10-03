mod multi_threaded;
mod simple;
mod single_threaded;

pub use self::{
    multi_threaded::{MainThreadExecutor, MultiThreadedExecutor},
    simple::SimpleExecutor,
    single_threaded::SingleThreadedExecutor,
};

use fixedbitset::FixedBitSet;

use crate::{
    schedule::{BoxedCondition, NodeId},
    system::BoxedSystem,
    world::World,
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
    Simple,
    /// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
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
    pub(super) system_ids: Vec<NodeId>,
    /// Indexed by system node id.
    pub(super) systems: Vec<BoxedSystem>,
    /// Indexed by system node id.
    pub(super) system_conditions: Vec<Vec<BoxedCondition>>,
    /// Indexed by system node id.
    /// Number of systems that the system immediately depends on.
    pub(super) system_dependencies: Vec<usize>,
    /// Indexed by system node id.
    /// List of systems that immediately depend on the system.
    pub(super) system_dependents: Vec<Vec<usize>>,
    /// Indexed by system node id.
    /// List of sets containing the system that have conditions
    pub(super) sets_with_conditions_of_systems: Vec<FixedBitSet>,
    /// List of system set node ids.
    pub(super) set_ids: Vec<NodeId>,
    /// Indexed by system set node id.
    pub(super) set_conditions: Vec<Vec<BoxedCondition>>,
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

/// Instructs the executor to call [`System::apply_deferred`](crate::system::System::apply_deferred)
/// on the systems that have run but not applied their [`Deferred`](crate::system::Deferred) system parameters
/// (like [`Commands`](crate::prelude::Commands)) or other system buffers.
///
/// ## Scheduling
///
/// `apply_deferred` systems are scheduled *by default*
/// - later in the same schedule run (for example, if a system with `Commands` param
///   is scheduled in `Update`, all the changes will be visible in `PostUpdate`)
/// - between systems with dependencies if the dependency
///   [has deferred buffers](crate::system::System::has_deferred)
///   (if system `bar` directly or indirectly depends on `foo`, and `foo` uses `Commands` param,
///   changes to the world in `foo` will be visible in `bar`)
///
/// ## Notes
/// - This function (currently) does nothing if it's called manually or wrapped inside a [`PipeSystem`](crate::system::PipeSystem).
/// - Modifying a [`Schedule`](super::Schedule) may change the order buffers are applied.
#[doc(alias = "apply_system_buffers")]
#[allow(unused_variables)]
pub fn apply_deferred(world: &mut World) {}

/// Returns `true` if the [`System`](crate::system::System) is an instance of [`apply_deferred`].
pub(super) fn is_apply_deferred(system: &BoxedSystem) -> bool {
    use crate::system::IntoSystem;
    // deref to use `System::type_id` instead of `Any::type_id`
    system.as_ref().type_id() == apply_deferred.system_type_id()
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

    use crate::{
        system::{ReadOnlySystem, System},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    };

    /// # Safety
    /// See `System::run_unsafe`.
    #[inline(never)]
    pub(super) unsafe fn run_unsafe(
        system: &mut dyn System<In = (), Out = ()>,
        world: UnsafeWorldCell,
    ) {
        system.run_unsafe((), world);
        black_box(());
    }

    /// # Safety
    /// See `ReadOnlySystem::run_unsafe`.
    #[inline(never)]
    pub(super) unsafe fn readonly_run_unsafe<O: 'static>(
        system: &mut dyn ReadOnlySystem<In = (), Out = O>,
        world: UnsafeWorldCell,
    ) -> O {
        black_box(system.run_unsafe((), world))
    }

    #[inline(never)]
    pub(super) fn run(system: &mut dyn System<In = (), Out = ()>, world: &mut World) {
        system.run((), world);
        black_box(());
    }

    #[inline(never)]
    pub(super) fn readonly_run<O: 'static>(
        system: &mut dyn ReadOnlySystem<In = (), Out = O>,
        world: &mut World,
    ) -> O {
        black_box(system.run((), world))
    }
}

#[macro_export]
/// Emits a warning about system being skipped.
macro_rules! warn_system_skipped {
    ($ty:literal, $sys:expr) => {
        bevy_utils::tracing::warn!(
            "{} {} was skipped due to inaccessible system parameters.",
            $ty,
            $sys
        )
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        prelude::{IntoSystemConfigs, IntoSystemSetConfigs, Resource, Schedule, SystemSet},
        schedule::ExecutorKind,
        system::{Commands, In, IntoSystem, Res},
        world::World,
    };

    #[derive(Resource)]
    struct R1;

    #[derive(Resource)]
    struct R2;

    const EXECUTORS: [ExecutorKind; 3] = [
        ExecutorKind::Simple,
        ExecutorKind::SingleThreaded,
        ExecutorKind::MultiThreaded,
    ];

    #[test]
    fn invalid_system_param_skips() {
        for executor in EXECUTORS {
            invalid_system_param_skips_core(executor);
        }
    }

    fn invalid_system_param_skips_core(executor: ExecutorKind) {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.set_executor_kind(executor);
        schedule.add_systems(
            (
                // Combined systems get skipped together.
                (|mut commands: Commands| {
                    commands.insert_resource(R1);
                })
                .pipe(|_: In<()>, _: Res<R1>| {}),
                // This system depends on a system that is always skipped.
                |mut commands: Commands| {
                    commands.insert_resource(R2);
                },
            )
                .chain(),
        );
        schedule.run(&mut world);
        assert!(world.get_resource::<R1>().is_none());
        assert!(world.get_resource::<R2>().is_some());
    }

    #[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone)]
    struct S1;

    #[test]
    fn invalid_condition_param_skips_system() {
        for executor in EXECUTORS {
            invalid_condition_param_skips_system_core(executor);
        }
    }

    fn invalid_condition_param_skips_system_core(executor: ExecutorKind) {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.set_executor_kind(executor);
        schedule.configure_sets(S1.run_if(|_: Res<R1>| true));
        schedule.add_systems((
            // System gets skipped if system set run conditions fail validation.
            (|mut commands: Commands| {
                commands.insert_resource(R1);
            })
            .in_set(S1),
            // System gets skipped if run conditions fail validation.
            (|mut commands: Commands| {
                commands.insert_resource(R2);
            })
            .run_if(|_: Res<R2>| true),
        ));
        schedule.run(&mut world);
        assert!(world.get_resource::<R1>().is_none());
        assert!(world.get_resource::<R2>().is_none());
    }
}
