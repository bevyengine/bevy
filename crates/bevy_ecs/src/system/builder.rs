use std::marker::PhantomData;

use bevy_utils::all_tuples;

use crate::{archetype::ArchetypeGeneration, prelude::World};

use super::{BuildableSystemParam, FunctionSystem, SystemMeta, SystemParam, SystemParamFunction};

/// Builder struct used to construct state for [`SystemParam`] passed to a system.
pub struct SystemBuilder<'w, T: SystemParam = ()> {
    meta: SystemMeta,
    state: T::State,
    world: &'w mut World,
}

impl<'w, T: SystemParam> SystemBuilder<'w, T> {
    /// Construct a new builder with the default state for `T`
    pub fn new(world: &'w mut World) -> Self {
        let mut meta = SystemMeta::new::<T>();
        Self {
            state: T::init_state(world, &mut meta),
            meta,
            world,
        }
    }

    /// Construct the final system with the built params
    pub fn build<F, Marker>(self, func: F) -> FunctionSystem<Marker, F>
    where
        F: SystemParamFunction<Marker, Param = T>,
    {
        FunctionSystem {
            func,
            param_state: Some(self.state),
            system_meta: self.meta,
            world_id: Some(self.world.id()),
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

macro_rules! impl_system_builder {
    ($($curr: ident),*) => {
        impl<'w, $($curr: SystemParam,)*> SystemBuilder<'w, ($($curr,)*)> {
            /// Add `T` as a parameter built from the world
            pub fn param<T: SystemParam>(mut self) -> SystemBuilder<'w, ($($curr,)* T,)> {
                #[allow(non_snake_case)]
                let ($($curr,)*) = self.state;
                SystemBuilder {
                    state: ($($curr,)* T::init_state(self.world, &mut self.meta),),
                    meta: self.meta,
                    world: self.world,
                }
            }

            /// Add `T` as a parameter built with the `func`
            pub fn builder<T: BuildableSystemParam>(
                mut self,
                func: impl FnOnce(&mut T::Builder<'_>),
            ) -> SystemBuilder<'w, ($($curr,)* T,)> {
                #[allow(non_snake_case)]
                let ($($curr,)*) = self.state;
                SystemBuilder {
                    state: ($($curr,)* T::build(self.world, &mut self.meta, func),),
                    meta: self.meta,
                    world: self.world,
                }
            }
        }
    };
}

all_tuples!(impl_system_builder, 0, 15, P);

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::prelude::{Component, Query};
    use crate::system::{Local, RunSystemOnce};

    use super::*;

    #[derive(Component)]
    struct A;

    fn local_system(local: Local<u64>) -> u64 {
        *local
    }

    fn query_system(query: Query<()>) -> usize {
        query.iter().count()
    }

    fn multi_param_system(a: Local<u64>, b: Local<u64>) -> u64 {
        *a + *b + 1
    }

    #[test]
    fn local_builder() {
        let mut world = World::new();

        let system = SystemBuilder::<()>::new(&mut world)
            .builder::<Local<u64>>(|x| *x = 10)
            .build(local_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 10);
    }

    #[test]
    fn query_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = SystemBuilder::<()>::new(&mut world)
            .builder::<Query<()>>(|query| {
                query.with::<A>();
            })
            .build(query_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }

    #[test]
    fn multi_param_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = SystemBuilder::<()>::new(&mut world)
            .param::<Local<u64>>()
            .param::<Local<u64>>()
            .build(multi_param_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }
}
