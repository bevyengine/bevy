#![allow(deprecated)]

#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use fixedbitset::FixedBitSet;

use crate::{
    schedule::{ExecutorKind, SingleThreadedExecutor, SystemExecutor, SystemSchedule},
    world::World,
};

/// A variant of [`SingleThreadedExecutor`](crate::schedule::SingleThreadedExecutor) that calls
/// [`apply_deferred`](crate::system::System::apply_deferred) immediately after running each system.
#[derive(Default)]
#[deprecated(
    since = "0.14.0",
    note = "The SimpleExecutor now is identical to the SingleThreaded executor. Use the single-threaded executor instead."
)]
pub struct SimpleExecutor {
    executor: SingleThreadedExecutor,
}

impl SystemExecutor for SimpleExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Simple
    }

    fn init(&mut self, schedule: &SystemSchedule) {
        self.executor.init(schedule);
    }

    fn run(
        &mut self,
        schedule: &mut SystemSchedule,
        world: &mut World,
        _skip_systems: Option<&FixedBitSet>,
    ) {
        self.executor.run(schedule, world, _skip_systems);
    }

    fn set_apply_final_deferred(&mut self, apply: bool) {
        self.executor.set_apply_final_deferred(apply);
    }
}

impl SimpleExecutor {
    /// Creates a new simple executor for use in a [`Schedule`](crate::schedule::Schedule).
    /// This calls each system in order and immediately calls [`System::apply_deferred`](crate::system::System::apply_deferred).
    pub const fn new() -> Self {
        Self {
            executor: SingleThreadedExecutor::new(),
        }
    }
}
