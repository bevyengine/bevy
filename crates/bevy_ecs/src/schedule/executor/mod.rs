mod multi_threaded;
mod simple;
mod single_threaded;

pub use self::multi_threaded::{MainThreadExecutor, MultiThreadedExecutor};
pub use self::simple::SimpleExecutor;
pub use self::single_threaded::SingleThreadedExecutor;

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
    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World);
    fn set_apply_final_deferred(&mut self, value: bool);
}

/// Specifies how a [`Schedule`](super::Schedule) will be run.
///
/// The default depends on the target platform:
///  - [`SingleThreaded`](ExecutorKind::SingleThreaded) on WASM.
///  - [`MultiThreaded`](ExecutorKind::MultiThreaded) everywhere else.
#[derive(PartialEq, Eq, Default, Debug, Copy, Clone)]
pub enum ExecutorKind {
    /// Runs the schedule using a single thread.
    ///
    /// Useful if you're dealing with a single-threaded environment, saving your threads for
    /// other things, or just trying minimize overhead.
    #[cfg_attr(any(target_arch = "wasm32", not(feature = "multi-threaded")), default)]
    SingleThreaded,
    /// Like [`SingleThreaded`](ExecutorKind::SingleThreaded) but calls [`apply_deferred`](crate::system::System::apply_deferred)
    /// immediately after running each system.
    Simple,
    /// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
    #[cfg_attr(all(not(target_arch = "wasm32"), feature = "multi-threaded"), default)]
    MultiThreaded,
}

/// Holds systems and conditions of a [`Schedule`](super::Schedule) sorted in topological order
/// (along with dependency information for multi-threaded execution).
///
/// Since the arrays are sorted in the same order, elements are referenced by their index.
/// [`FixedBitSet`] is used as a smaller, more efficient substitute of `HashSet<usize>`.
#[derive(Default)]
pub struct SystemSchedule {
    pub(super) systems: Vec<BoxedSystem>,
    pub(super) system_conditions: Vec<Vec<BoxedCondition>>,
    pub(super) set_conditions: Vec<Vec<BoxedCondition>>,
    pub(super) system_ids: Vec<NodeId>,
    pub(super) set_ids: Vec<NodeId>,
    pub(super) system_dependencies: Vec<usize>,
    pub(super) system_dependents: Vec<Vec<usize>>,
    pub(super) sets_with_conditions_of_systems: Vec<FixedBitSet>,
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
/// on the systems that have run but not applied their [`Deferred`](crate::system::Deferred) system parameters (like [`Commands`](crate::prelude::Commands)) or other system buffers.
///
/// **Notes**
/// - This function (currently) does nothing if it's called manually or wrapped inside a [`PipeSystem`](crate::system::PipeSystem).
/// - Modifying a [`Schedule`](super::Schedule) may change the order buffers are applied.
#[doc(alias = "apply_system_buffers")]
#[allow(unused_variables)]
pub fn apply_deferred(world: &mut World) {}

/// Returns `true` if the [`System`](crate::system::System) is an instance of [`apply_deferred`].
pub(super) fn is_apply_deferred(system: &BoxedSystem) -> bool {
    use std::any::Any;
    // deref to use `System::type_id` instead of `Any::type_id`
    system.as_ref().type_id() == apply_deferred.type_id()
}
