mod multi_threaded;
mod simple;
mod single_threaded;

pub use self::multi_threaded::MultiThreadedExecutor;
pub use self::simple::SimpleExecutor;
pub use self::single_threaded::SingleThreadedExecutor;

use std::any::{Any, TypeId};
use std::cell::RefCell;

use fixedbitset::FixedBitSet;

use crate::{
    schedule_v3::{BoxedCondition, NodeId},
    system::{BoxedSystem, IntoSystem},
    world::World,
};

/// Types that can run a [`SystemSchedule`] on a [`World`].
pub(super) trait SystemExecutor: Send + Sync {
    fn init(&mut self, schedule: &SystemSchedule);
    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World);
}

/// Controls how a [`Schedule`] will be run.
pub enum ExecutorKind {
    /// Runs the schedule using a single thread.
    SingleThreaded,
    /// Like [`SingleThreaded`](ExecutorKind::SingleThreaded) but calls [`apply_buffers`](crate::system::System::apply_buffers)
    /// immediately after running each system.
    Simple,
    /// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
    MultiThreaded,
}

#[derive(Default)]
pub(super) struct SystemSchedule {
    pub(super) systems: Vec<RefCell<BoxedSystem>>,
    pub(super) system_conditions: Vec<RefCell<Vec<BoxedCondition>>>,
    pub(super) set_conditions: Vec<RefCell<Vec<BoxedCondition>>>,
    pub(super) system_ids: Vec<NodeId>,
    pub(super) set_ids: Vec<NodeId>,
    pub(super) system_deps: Vec<(usize, Vec<usize>)>,
    pub(super) sets_of_systems: Vec<FixedBitSet>,
    pub(super) sets_of_sets: Vec<FixedBitSet>,
    pub(super) systems_of_sets: Vec<FixedBitSet>,
}

impl SystemSchedule {
    pub const fn new() -> Self {
        Self {
            systems: Vec::new(),
            system_conditions: Vec::new(),
            set_conditions: Vec::new(),
            system_ids: Vec::new(),
            set_ids: Vec::new(),
            system_deps: Vec::new(),
            sets_of_systems: Vec::new(),
            sets_of_sets: Vec::new(),
            systems_of_sets: Vec::new(),
        }
    }
}

// SAFETY: MultiThreadedExecutor does not alias RefCell instances
unsafe impl Sync for SystemSchedule {}

/// Instructs the executor to call [`apply_buffers`](crate::system::System::apply_buffers)
/// on the systems that have run but not applied their buffers.
///
/// **Notes**
/// - This function (currently) does nothing if it's called manually or wrapped inside a [`PipeSystem`](crate::system::PipeSystem).
/// - Modifying a [`Schedule`] may change the order buffers are applied.
#[allow(unused_variables)]
pub fn apply_system_buffers(world: &mut World) {}

/// Returns `true` if the [`System`] is an instance of [`apply_system_buffers`].
pub(super) fn is_apply_system_buffers(system: &BoxedSystem) -> bool {
    fn get_type_id<T: Any>(_: &T) -> TypeId {
        TypeId::of::<T>()
    }
    let type_id = get_type_id(&IntoSystem::into_system(apply_system_buffers));
    (&*system as &dyn Any).type_id() == type_id
}
