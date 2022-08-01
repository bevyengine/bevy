mod multi_threaded;
mod simple;
mod single_threaded;

pub use self::multi_threaded::MultiThreadedRunner;
pub use self::simple::SimpleRunner;
pub use self::single_threaded::SingleThreadedRunner;

use crate::{
    schedule::SystemLabel,
    schedule_v3::{BoxedRunCondition, NodeId, Schedule, Systems},
    system::{AsSystemLabel, BoxedSystem},
    world::World,
};

/// Types that can run a [`Schedule`] on a [`World`].
pub(crate) trait Runner: Send + Sync {
    fn init(&mut self, schedule: &Schedule);
    fn run(&mut self, schedule: &mut Schedule, world: &mut World);
}

/// Internal resource written by [`apply_system_buffers`] to pass a signal back up to the [`Runner`].
pub(crate) struct RunnerApplyBuffers(pub(crate) bool);

impl Default for RunnerApplyBuffers {
    fn default() -> Self {
        Self(false)
    }
}

/// Signals the runner to call [`System::apply_buffers`](crate::system::System::apply_buffers) for all
/// systems in the running [`Schedule`] that have run but not applied their buffers.
///
/// **Note**: System buffers are applied in a topological order.
pub fn apply_system_buffers(world: &mut World) {
    let mut flag = world.resource_mut::<RunnerApplyBuffers>();
    debug_assert!(!flag.0);
    flag.0 = true;
    // this is here because this system will be called often
    world.check_change_ticks();
}
