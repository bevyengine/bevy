use crate::{
    change_detection::Tick,
    prelude::World,
    query::FilteredAccessSet,
    system::{ExclusiveSystemParam, ReadOnlySystemParam, SystemMeta, SystemParam},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_utils::prelude::DebugName;
use derive_more::derive::{Display, Into};

use super::SharedStates;

/// [`SystemParam`] that returns the name of the system which it is used in.
///
/// This is not a reliable identifier, it is more so useful for debugging or logging.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::system::SystemName;
/// # use bevy_ecs::system::SystemParam;
///
/// #[derive(SystemParam)]
/// struct Logger {
///     system_name: SystemName,
/// }
///
/// impl Logger {
///     fn log(&mut self, message: &str) {
///         eprintln!("{}: {}", self.system_name, message);
///     }
/// }
///
/// fn system1(mut logger: Logger) {
///     // Prints: "crate_name::mod_name::system1: Hello".
///     logger.log("Hello");
/// }
/// ```
#[derive(Debug, Into, Display)]
pub struct SystemName(DebugName);

impl SystemName {
    /// Gets the name of the system.
    pub fn name(&self) -> DebugName {
        self.0.clone()
    }
}

// SAFETY: no component value access
unsafe impl SystemParam for SystemName {
    type State = ();
    type Item<'w, 's> = SystemName;

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {}

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        SystemName(system_meta.name.clone())
    }
}

// SAFETY: Only reads internal system state
unsafe impl ReadOnlySystemParam for SystemName {}

impl ExclusiveSystemParam for SystemName {
    type State = ();
    type Item<'s> = SystemName;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    fn get_param<'s>(_state: &'s mut Self::State, system_meta: &SystemMeta) -> Self::Item<'s> {
        SystemName(system_meta.name.clone())
    }
}

#[cfg(test)]
#[cfg(all(feature = "trace", feature = "debug"))]
mod tests {
    use crate::{
        system::{IntoSystem, RunSystemOnce, SystemName},
        world::World,
    };
    use alloc::{borrow::ToOwned, string::String};

    #[test]
    fn test_system_name_regular_param() {
        fn testing(name: SystemName) -> String {
            name.name().as_string()
        }

        let mut world = World::default();
        let id = world.register_system(testing);
        let name = world.run_system(id).unwrap();
        assert!(name.ends_with("testing"));
    }

    #[test]
    fn test_system_name_exclusive_param() {
        fn testing(_world: &mut World, name: SystemName) -> String {
            name.name().as_string()
        }

        let mut world = World::default();
        let id = world.register_system(testing);
        let name = world.run_system(id).unwrap();
        assert!(name.ends_with("testing"));
    }

    #[test]
    fn test_closure_system_name_regular_param() {
        let mut world = World::default();
        let system =
            IntoSystem::into_system(|name: SystemName| name.name().to_owned()).with_name("testing");
        let name = world.run_system_once(system).unwrap().as_string();
        assert_eq!(name, "testing");
    }

    #[test]
    fn test_exclusive_closure_system_name_regular_param() {
        let mut world = World::default();
        let system =
            IntoSystem::into_system(|_world: &mut World, name: SystemName| name.name().to_owned())
                .with_name("testing");
        let name = world.run_system_once(system).unwrap().as_string();
        assert_eq!(name, "testing");
    }
}
