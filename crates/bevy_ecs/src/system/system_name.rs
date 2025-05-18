use crate::{
    component::Tick,
    prelude::World,
    system::{ExclusiveSystemParam, ReadOnlySystemParam, SystemMeta, SystemParam},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use alloc::borrow::Cow;
use core::ops::Deref;
use derive_more::derive::{AsRef, Display, Into};

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
/// struct Logger<'s> {
///     system_name: SystemName<'s>,
/// }
///
/// impl<'s> Logger<'s> {
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
#[derive(Debug, Into, Display, AsRef)]
#[as_ref(str)]
pub struct SystemName<'s>(&'s str);

impl<'s> SystemName<'s> {
    /// Gets the name of the system.
    pub fn name(&self) -> &str {
        self.0
    }
}

impl<'s> Deref for SystemName<'s> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.name()
    }
}

// SAFETY: no component value access
unsafe impl SystemParam for SystemName<'_> {
    type State = Cow<'static, str>;
    type Item<'w, 's> = SystemName<'s>;

    fn init_state(_world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.name.clone()
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        name: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        SystemName(name)
    }
}

// SAFETY: Only reads internal system state
unsafe impl<'s> ReadOnlySystemParam for SystemName<'s> {}

impl ExclusiveSystemParam for SystemName<'_> {
    type State = Cow<'static, str>;
    type Item<'s> = SystemName<'s>;

    fn init(_world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.name.clone()
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        SystemName(state)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        system::{IntoSystem, RunSystemOnce, SystemName},
        world::World,
    };
    use alloc::{borrow::ToOwned, string::String};

    #[test]
    fn test_system_name_regular_param() {
        fn testing(name: SystemName) -> String {
            name.name().to_owned()
        }

        let mut world = World::default();
        let id = world.register_system(testing);
        let name = world.run_system(id).unwrap();
        assert!(name.ends_with("testing"));
    }

    #[test]
    fn test_system_name_exclusive_param() {
        fn testing(_world: &mut World, name: SystemName) -> String {
            name.name().to_owned()
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
        let name = world.run_system_once(system).unwrap();
        assert_eq!(name, "testing");
    }

    #[test]
    fn test_exclusive_closure_system_name_regular_param() {
        let mut world = World::default();
        let system =
            IntoSystem::into_system(|_world: &mut World, name: SystemName| name.name().to_owned())
                .with_name("testing");
        let name = world.run_system_once(system).unwrap();
        assert_eq!(name, "testing");
    }
}
