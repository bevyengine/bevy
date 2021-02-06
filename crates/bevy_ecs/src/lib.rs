mod core;
mod resource;
mod schedule;
mod system;

pub use crate::core::*;
pub use bevy_ecs_macros::*;
pub use lazy_static;
pub use resource::*;
pub use schedule::*;
pub use system::{Query, *};

pub mod prelude {
    pub use crate::{
        core::WorldBuilderSource,
        resource::{ChangedRes, FromResources, Local, NonSend, Res, ResMut, Resource, Resources},
        schedule::{
            ExclusiveSystemDescriptorCoercion, ParallelSystemDescriptorCoercion,
            ReportExecutionOrderAmbiguities, RunOnce, Schedule, Stage, State, StateStage,
            SystemSet, SystemStage,
        },
        system::{Commands, ExclusiveSystem, IntoExclusiveSystem, IntoSystem, Query, System},
        Added, Bundle, Changed, Component, Entity, Flags, In, IntoChainSystem, Mut, Mutated, Or,
        QuerySet, Ref, RefMut, ShouldRun, With, Without, World,
    };
}
