pub mod resource;
pub mod schedule;
pub mod profiler;

mod system;
mod system_fn;
mod system_fn_types;

pub use bit_set;
pub use system::*;
pub use system_fn::*;
pub use system_fn_types::{Res, ResMut};

pub mod prelude {
    pub use crate::{
        bit_set::BitSet,
        resource::{ResourceSet, Resources},
        schedule::{Executor, Runnable, Schedulable, Schedule},
        IntoSystem, SimpleQuery as Query, Res, ResMut, System, SystemBuilder,
    };
}
