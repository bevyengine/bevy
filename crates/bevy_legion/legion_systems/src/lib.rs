#![feature(trace_macros)]
pub mod resource;
pub mod schedule;

mod system_fn;
mod system_fn_types;
mod system;

pub use bit_set;
pub use system::*;
pub use system_fn::*;

pub mod prelude {
    pub use crate::{
        bit_set::BitSet,
        // aliased preparedread and preparedwrite used by system_fn
        resource::{ResourceSet, Resources, PreparedRead as Resource, PreparedWrite as ResourceMut},
        schedule::{Executor, Runnable, Schedulable, Schedule},
        System, SystemBuilder,
        into_resource_system, into_resource_for_each_system,
        IntoSystem
    };
}
