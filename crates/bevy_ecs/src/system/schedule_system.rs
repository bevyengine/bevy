use alloc::{borrow::Cow, vec::Vec};
use core::any::TypeId;

use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    query::{Access, FilteredAccessSet},
    result::Result,
    schedule::InternedSystemSet,
    system::{input::SystemIn, BoxedSystem, System},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

/// A type which wraps and unifies the different sorts of systems that can be added to a schedule.
pub enum ScheduleSystem {
    /// A system that does not return a result.
    Infallible(BoxedSystem<(), ()>),
    /// A system that does return a result.
    Fallible(BoxedSystem<(), Result>),
}

impl System for ScheduleSystem {
    type In = ();
    type Out = Result;

    #[inline(always)]
    fn name(&self) -> Cow<'static, str> {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.name(),
            ScheduleSystem::Fallible(inner_system) => inner_system.name(),
        }
    }

    #[inline(always)]
    fn type_id(&self) -> TypeId {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.type_id(),
            ScheduleSystem::Fallible(inner_system) => inner_system.type_id(),
        }
    }

    #[inline(always)]
    fn component_access(&self) -> &Access<ComponentId> {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.component_access(),
            ScheduleSystem::Fallible(inner_system) => inner_system.component_access(),
        }
    }

    #[inline(always)]
    fn component_access_set(&self) -> &FilteredAccessSet<ComponentId> {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.component_access_set(),
            ScheduleSystem::Fallible(inner_system) => inner_system.component_access_set(),
        }
    }

    #[inline(always)]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.archetype_component_access(),
            ScheduleSystem::Fallible(inner_system) => inner_system.archetype_component_access(),
        }
    }

    #[inline(always)]
    fn is_exclusive(&self) -> bool {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.is_exclusive(),
            ScheduleSystem::Fallible(inner_system) => inner_system.is_exclusive(),
        }
    }

    #[inline(always)]
    fn has_deferred(&self) -> bool {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.has_deferred(),
            ScheduleSystem::Fallible(inner_system) => inner_system.has_deferred(),
        }
    }

    #[inline(always)]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        match self {
            ScheduleSystem::Infallible(inner_system) => {
                inner_system.run_unsafe(input, world);
                Ok(())
            }
            ScheduleSystem::Fallible(inner_system) => inner_system.run_unsafe(input, world),
        }
    }

    #[inline(always)]
    fn run(&mut self, input: SystemIn<'_, Self>, world: &mut World) -> Self::Out {
        match self {
            ScheduleSystem::Infallible(inner_system) => {
                inner_system.run(input, world);
                Ok(())
            }
            ScheduleSystem::Fallible(inner_system) => inner_system.run(input, world),
        }
    }

    #[inline(always)]
    fn apply_deferred(&mut self, world: &mut World) {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.apply_deferred(world),
            ScheduleSystem::Fallible(inner_system) => inner_system.apply_deferred(world),
        }
    }

    #[inline(always)]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.queue_deferred(world),
            ScheduleSystem::Fallible(inner_system) => inner_system.queue_deferred(world),
        }
    }

    #[inline(always)]
    fn is_send(&self) -> bool {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.is_send(),
            ScheduleSystem::Fallible(inner_system) => inner_system.is_send(),
        }
    }

    #[inline(always)]
    unsafe fn validate_param_unsafe(&mut self, world: UnsafeWorldCell) -> bool {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.validate_param_unsafe(world),
            ScheduleSystem::Fallible(inner_system) => inner_system.validate_param_unsafe(world),
        }
    }

    #[inline(always)]
    fn initialize(&mut self, world: &mut World) {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.initialize(world),
            ScheduleSystem::Fallible(inner_system) => inner_system.initialize(world),
        }
    }

    #[inline(always)]
    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        match self {
            ScheduleSystem::Infallible(inner_system) => {
                inner_system.update_archetype_component_access(world);
            }
            ScheduleSystem::Fallible(inner_system) => {
                inner_system.update_archetype_component_access(world);
            }
        }
    }

    #[inline(always)]
    fn check_change_tick(&mut self, change_tick: Tick) {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.check_change_tick(change_tick),
            ScheduleSystem::Fallible(inner_system) => inner_system.check_change_tick(change_tick),
        }
    }

    #[inline(always)]
    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.default_system_sets(),
            ScheduleSystem::Fallible(inner_system) => inner_system.default_system_sets(),
        }
    }

    #[inline(always)]
    fn get_last_run(&self) -> Tick {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.get_last_run(),
            ScheduleSystem::Fallible(inner_system) => inner_system.get_last_run(),
        }
    }

    #[inline(always)]
    fn set_last_run(&mut self, last_run: Tick) {
        match self {
            ScheduleSystem::Infallible(inner_system) => inner_system.set_last_run(last_run),
            ScheduleSystem::Fallible(inner_system) => inner_system.set_last_run(last_run),
        }
    }
}
