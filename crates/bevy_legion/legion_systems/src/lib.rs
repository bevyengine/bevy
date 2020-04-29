#![feature(trace_macros)]
pub mod resource;
pub mod schedule;

mod system;
mod system_fn;
mod system_fn_types;

pub use bit_set;
pub use system::*;
pub use system_fn::*;
pub use system_fn_types::{ResourceMut, Resource}; 

pub mod prelude {
    pub use crate::{
        bit_set::BitSet,
        into_resource_for_each_system,
        into_resource_system,
        // aliased preparedread and preparedwrite used by system_fn
        resource::{
            ResourceSet, Resources,
        },
        schedule::{Executor, Runnable, Schedulable, Schedule},
        IntoSystem,
        System,
        SystemBuilder,
        Resource, ResourceMut,
    };
}
