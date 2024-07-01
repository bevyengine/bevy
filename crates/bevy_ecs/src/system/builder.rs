use bevy_utils::all_tuples;

use super::{
    BuildableSystemParam, FunctionSystem, Local, Res, ResMut, Resource, SystemMeta, SystemParam,
    SystemParamFunction, SystemState,
};
use crate::prelude::{FromWorld, Query, World};
use crate::query::{QueryData, QueryFilter};

/// Builder struct used to construct state for [`SystemParam`] passed to a system.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs_macros::SystemParam;
/// # use bevy_ecs::system::RunSystemOnce;
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # #[derive(Component)]
/// # struct B;
/// #
/// # #[derive(Resource)]
/// # struct R;
/// #
/// # #[derive(SystemParam)]
/// # struct MyParam;
/// #
/// # let mut world = World::new();
/// # world.insert_resource(R);
///
/// fn my_system(res: Res<R>, query: Query<&A>, param: MyParam) {
///     // ...
/// }
///
/// // Create a builder from the world, helper methods exist to add `SystemParam`,
/// // alternatively use `.param::<T>()` for any other `SystemParam` types.
/// let system = SystemBuilder::<()>::new(&mut world)
///     .resource::<R>()
///     .query::<&A>()
///     .param::<MyParam>()
///     .build(my_system);
///
/// // Parameters that the builder is initialised with will appear first in the arguments.
/// let system = SystemBuilder::<(Res<R>, Query<&A>)>::new(&mut world)
///     .param::<MyParam>()
///     .build(my_system);
///
/// // Parameters that implement `BuildableSystemParam` can use `.builder::<T>()` to build in place.
/// let system = SystemBuilder::<()>::new(&mut world)
///     .resource::<R>()
///     .builder::<Query<&A>>(|builder| { builder.with::<B>(); })
///     .param::<MyParam>()
///     .build(my_system);
///
/// world.run_system_once(system);
///```
pub struct SystemBuilder<'w, T: SystemParam = ()> {
    pub(crate) meta: SystemMeta,
    pub(crate) state: T::State,
    pub(crate) world: &'w mut World,
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

    /// Construct the a system with the built params
    pub fn build<F, Marker>(self, func: F) -> FunctionSystem<Marker, F>
    where
        F: SystemParamFunction<Marker, Param = T>,
    {
        FunctionSystem::from_builder(self, func)
    }

    /// Return the constructed [`SystemState`]
    pub fn state(self) -> SystemState<T> {
        SystemState::from_builder(self)
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

            /// Helper method for reading a [`Resource`] as a param, equivalent to `.param::<Res<T>>()`
            pub fn resource<T: Resource>(self) -> SystemBuilder<'w,  ($($curr,)* Res<'static, T>,)> {
                self.param::<Res<T>>()
            }

            /// Helper method for mutably accessing a [`Resource`] as a param, equivalent to `.param::<ResMut<T>>()`
            pub fn resource_mut<T: Resource>(self) -> SystemBuilder<'w,  ($($curr,)* ResMut<'static, T>,)> {
                self.param::<ResMut<T>>()
            }

            /// Helper method for adding a [`Local`] as a param, equivalent to `.param::<Local<T>>()`
            pub fn local<T: Send + FromWorld>(self) -> SystemBuilder<'w,  ($($curr,)* Local<'static, T>,)> {
                self.param::<Local<T>>()
            }

            /// Helper method for adding a [`Query`] as a param, equivalent to `.param::<Query<D>>()`
            pub fn query<D: QueryData>(self) -> SystemBuilder<'w,  ($($curr,)* Query<'static, 'static, D, ()>,)> {
                self.query_filtered::<D, ()>()
            }

            /// Helper method for adding a filtered [`Query`] as a param, equivalent to `.param::<Query<D, F>>()`
            pub fn query_filtered<D: QueryData, F: QueryFilter>(self) -> SystemBuilder<'w,  ($($curr,)* Query<'static, 'static, D, F>,)> {
                self.param::<Query<D, F>>()
            }

            /// Add `T` as a parameter built with the given function
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
            .local::<u64>()
            .param::<Local<u64>>()
            .build(multi_param_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }
}
