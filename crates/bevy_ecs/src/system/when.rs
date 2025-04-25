use super::{
    ReadOnlySystemParam, Res, ResMut, SystemMeta, SystemParam, SystemParamValidationError,
};
use crate::{
    component::{ComponentId, Tick},
    prelude::Resource,
    storage::ResourceData,
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

/// An alternative to types like [`Res`] that should `skip` instead of panic when they dont exist.
/// Unlike [`Option<Res<T>>`], this will cause the system to be skipped entirely if the resource does not exist.
/// # Example
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Resource)]
/// struct Foo;
/// 
/// fn skips_if_not_present(res: When<Foo>){}
/// ```
pub struct When<'a, T> {
    pub(crate) value: &'a T,
}

impl<T> std::ops::Deref for When<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for When<'a, T> {}

unsafe impl<'a, T: Resource> SystemParam for When<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = When<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Res::<'a, T>::init_state(world, system_meta)
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        unsafe {
            let value: Res<'w, T> = Res::get_param(state, system_meta, world, change_tick);
            When {
                value: value.into_inner(),
            }
        }
    }
    unsafe fn validate_param(
        &component_id: &Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to resource metadata.
        if unsafe { world.storages() }
            .resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
        {
            Ok(())
        } else {
            Err(SystemParamValidationError::skipped::<Self>(
                "Resource does not exist",
            ))
        }
    }
}

/// An alternative to types like [`ResMut`] that should `skip` instead of panic when they dont exist.
/// Unlike [`Option<ResMut<T>>`], this will cause the system to be skipped entirely if the resource does not exist.
/// # Example
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Resource)]
/// struct Foo;
/// 
/// fn skips_if_not_present(res: WhenMut<Foo>){}
/// ```
pub struct WhenMut<'a, T> {
    pub(crate) value: &'a mut T,
}

impl<T> std::ops::Deref for WhenMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> std::ops::DerefMut for WhenMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for WhenMut<'a, T> {}

unsafe impl<'a, T: Resource> SystemParam for WhenMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = WhenMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        ResMut::<'a, T>::init_state(world, system_meta)
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        unsafe {
            let value: ResMut<'w, T> = ResMut::get_param(state, system_meta, world, change_tick);
            WhenMut {
                value: value.into_inner(),
            }
        }
    }
    unsafe fn validate_param(
        &component_id: &Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to resource metadata.
        if unsafe { world.storages() }
            .resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
        {
            Ok(())
        } else {
            Err(SystemParamValidationError::skipped::<Self>(
                "Resource does not exist",
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    use super::{When, WhenMut};

    #[derive(Default, Resource)]
    struct Foo;

    #[test]
    #[should_panic]
    fn runs_when_present() {
        let mut world = World::new();
        world.insert_resource(Foo::default());
        let mut schedule = Schedule::default();
        schedule.add_systems(|_res: When<Foo>| panic!("will run"));
        schedule.run(&mut world);
    }
    #[test]
    fn skips_when_not_present() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(|_res: When<Foo>| panic!("will not run"));
        schedule.run(&mut world);
    }
    #[test]
    #[should_panic]
    fn runs_when_present_mut() {
        let mut world = World::new();
        world.insert_resource(Foo::default());
        let mut schedule = Schedule::default();
        schedule.add_systems(|_res: WhenMut<Foo>| panic!("will run"));
        schedule.run(&mut world);
    }
    #[test]
    fn skips_when_not_present_mut() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(|_res: WhenMut<Foo>| panic!("will not run"));
        schedule.run(&mut world);
    }
}
