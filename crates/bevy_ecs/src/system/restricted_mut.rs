use crate::{
    change_detection::{Mut, Tick},
    component::RestrictedAccess,
    entity::Entity,
    query::{FilteredAccessSet, QueryEntityError, QueryState, RestrictedWrite},
    system::{Query, SystemMeta, SystemParam, SystemParamValidationError},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

type RestrictedQuery<'w, 's, T> = Query<'w, 's, RestrictedWrite<T>>;

/// A [`SystemParam`] that provides authorized mutable access to components
/// marked with [`RestrictedAccess`].
///
/// Use this instead of `Query<&mut T>` for restricted components.
pub struct RestrictedMut<'w, 's, T: RestrictedAccess> {
    query: RestrictedQuery<'w, 's, T>,
}

impl<T: RestrictedAccess> RestrictedMut<'_, '_, T> {
    /// Gets authorized mutable access to the restricted component on `entity`.
    pub fn get_mut(&mut self, entity: Entity) -> Result<Mut<'_, T>, QueryEntityError> {
        self.query.get_mut(entity)
    }

    /// Mutates the restricted component on `entity` through `f`.
    pub fn modify<R>(
        &mut self,
        entity: Entity,
        f: impl FnOnce(&mut T) -> R,
    ) -> Result<R, QueryEntityError> {
        let mut component = self.get_mut(entity)?;
        Ok(f(&mut component))
    }

    /// Iterates over all matching restricted components with authorized mutable access.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Mut<'_, T>> + '_ {
        self.query.iter_mut()
    }
}

// SAFETY: `RestrictedMut` delegates all access registration, validation, and fetching to
// `Query<RestrictedWrite<T>>`, which registers authorized write access for `T`.
unsafe impl<T: RestrictedAccess> SystemParam for RestrictedMut<'_, '_, T> {
    type State = QueryState<RestrictedWrite<T>>;
    type Item<'w, 's> = RestrictedMut<'w, 's, T>;

    fn init_state(world: &mut World) -> Self::State {
        QueryState::new(world)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        <RestrictedQuery<'_, '_, T> as SystemParam>::init_access(
            state,
            system_meta,
            component_access_set,
            world,
        );
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
        Ok(RestrictedMut {
            // SAFETY: Access registration is delegated to `Query<RestrictedWrite<T>>`.
            query: unsafe {
                <RestrictedQuery<'w, 's, T> as SystemParam>::get_param(
                    state,
                    system_meta,
                    world,
                    change_tick,
                )?
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        component::{Component, RestrictedAccess},
        entity::Entity,
        resource::Resource,
        schedule::Schedule,
        system::{Res, RestrictedMut, SystemParam},
        world::World,
    };

    #[derive(Component, RestrictedAccess)]
    struct RestrictedCounter(u32);

    #[derive(Resource)]
    struct Target(Entity);

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn restricted_mut_systemparam_works() {
        assert_send_sync::<
            <RestrictedMut<'static, 'static, RestrictedCounter> as SystemParam>::State,
        >();

        fn increment(mut counters: RestrictedMut<RestrictedCounter>, target: Res<Target>) {
            counters
                .modify(target.0, |counter| counter.0 += 1)
                .expect("target should have RestrictedCounter");

            for mut counter in counters.iter_mut() {
                counter.0 += 1;
            }
        }

        let mut world = World::new();
        let entity = world.spawn(RestrictedCounter(1)).id();
        world.insert_resource(Target(entity));

        let mut schedule = Schedule::default();
        schedule.add_systems(increment);
        schedule.run(&mut world);

        let value = world
            .query::<&RestrictedCounter>()
            .single(&world)
            .expect("one counter should exist")
            .0;
        assert_eq!(value, 3);
    }
}
