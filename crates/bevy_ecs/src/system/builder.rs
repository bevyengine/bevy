use bevy_utils::{all_tuples, synccell::SyncCell};

use crate::{
    prelude::QueryBuilder,
    query::{QueryData, QueryFilter, QueryState},
    system::{
        system_param::{DynSystemParam, DynSystemParamState, Local, ParamSet, SystemParam},
        Query, SystemMeta,
    },
    world::{FromWorld, World},
};
use std::fmt::Debug;

use super::{init_query_param, Res, ResMut, Resource, SystemState};

/// A builder that can create a [`SystemParam`]
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs_macros::SystemParam;
/// # use bevy_ecs::system::{RunSystemOnce, ParamBuilder, LocalBuilder, QueryParamBuilder};
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
/// #
/// fn my_system(res: Res<R>, query: Query<&A>, param: MyParam) {
///     // ...
/// }
///
/// // To build a system, create a tuple of `SystemParamBuilder`s with a builder for each param.
/// // `ParamBuilder` can be used to build a parameter using its default initialization,
/// // and has helper methods to create typed builders.
/// let system = (
///     ParamBuilder,
///     ParamBuilder::query::<&A>(),
///     ParamBuilder::of::<MyParam>(),
/// )
///     .build_state(&mut world)
///     .build_system(my_system);
///
/// // Other implementations of `SystemParamBuilder` can be used to configure the parameters.
/// let system = (
///     ParamBuilder,
///     QueryParamBuilder::new::<&A, ()>(|builder| {
///         builder.with::<B>();
///     }),
///     ParamBuilder,
/// )
///     .build_state(&mut world)
///     .build_system(my_system);
///
/// fn single_parameter_system(local: Local<u64>) {
///     // ...
/// }
///
/// // Note that the builder for a system must be a tuple, even if there is only one parameter.
/// let system = (LocalBuilder(2),)
///     .build_state(&mut world)
///     .build_system(single_parameter_system);
///
/// world.run_system_once(system);
///```
///
/// # Safety
///
/// The implementor must ensure the following is true.
/// - [`SystemParamBuilder::build`] correctly registers all [`World`] accesses used
///   by [`SystemParam::get_param`] with the provided [`system_meta`](SystemMeta).
/// - None of the world accesses may conflict with any prior accesses registered
///   on `system_meta`.
///
/// Note that this depends on the implementation of [`SystemParam::get_param`],
/// so if `Self` is not a local type then you must call [`SystemParam::init_state`]
/// or another [`SystemParamBuilder::build`]
pub unsafe trait SystemParamBuilder<P: SystemParam>: Sized {
    /// Registers any [`World`] access used by this [`SystemParam`]
    /// and creates a new instance of this param's [`State`](SystemParam::State).
    fn build(self, world: &mut World, meta: &mut SystemMeta) -> P::State;

    /// Create a [`SystemState`] from a [`SystemParamBuilder`].
    /// To create a system, call [`SystemState::build_system`] on the result.
    fn build_state(self, world: &mut World) -> SystemState<P> {
        SystemState::from_builder(world, self)
    }
}

/// A [`SystemParamBuilder`] for any [`SystemParam`] that uses its default initialization.
#[derive(Default, Debug, Copy, Clone)]
pub struct ParamBuilder;

// SAFETY: Calls `SystemParam::init_state`
unsafe impl<P: SystemParam> SystemParamBuilder<P> for ParamBuilder {
    fn build(self, world: &mut World, meta: &mut SystemMeta) -> P::State {
        P::init_state(world, meta)
    }
}

impl ParamBuilder {
    /// Creates a [`SystemParamBuilder`] for any [`SystemParam`] that uses its default initialization.
    pub fn of<T: SystemParam>() -> impl SystemParamBuilder<T> {
        Self
    }

    /// Helper method for reading a [`Resource`] as a param, equivalent to `of::<Res<T>>()`
    pub fn resource<'w, T: Resource>() -> impl SystemParamBuilder<Res<'w, T>> {
        Self
    }

    /// Helper method for mutably accessing a [`Resource`] as a param, equivalent to `of::<ResMut<T>>()`
    pub fn resource_mut<'w, T: Resource>() -> impl SystemParamBuilder<ResMut<'w, T>> {
        Self
    }

    /// Helper method for adding a [`Local`] as a param, equivalent to `of::<Local<T>>()`
    pub fn local<'s, T: FromWorld + Send + 'static>() -> impl SystemParamBuilder<Local<'s, T>> {
        Self
    }

    /// Helper method for adding a [`Query`] as a param, equivalent to `of::<Query<D>>()`
    pub fn query<'w, 's, D: QueryData + 'static>() -> impl SystemParamBuilder<Query<'w, 's, D, ()>>
    {
        Self
    }

    /// Helper method for adding a filtered [`Query`] as a param, equivalent to `of::<Query<D, F>>()`
    pub fn query_filtered<'w, 's, D: QueryData + 'static, F: QueryFilter + 'static>(
    ) -> impl SystemParamBuilder<Query<'w, 's, D, F>> {
        Self
    }
}

// SAFETY: Calls `init_query_param`, just like `Query::init_state`.
unsafe impl<'w, 's, D: QueryData + 'static, F: QueryFilter + 'static>
    SystemParamBuilder<Query<'w, 's, D, F>> for QueryState<D, F>
{
    fn build(self, world: &mut World, system_meta: &mut SystemMeta) -> QueryState<D, F> {
        self.validate_world(world.id());
        init_query_param(world, system_meta, &self);
        self
    }
}

/// A [`SystemParamBuilder`] for a [`Query`].
pub struct QueryParamBuilder<T>(T);

impl<T> QueryParamBuilder<T> {
    /// Creates a [`SystemParamBuilder`] for a [`Query`] that accepts a callback to configure the [`QueryBuilder`].
    pub fn new<D: QueryData, F: QueryFilter>(f: T) -> Self
    where
        T: FnOnce(&mut QueryBuilder<D, F>),
    {
        Self(f)
    }
}

impl<'a, D: QueryData, F: QueryFilter>
    QueryParamBuilder<Box<dyn FnOnce(&mut QueryBuilder<D, F>) + 'a>>
{
    /// Creates a [`SystemParamBuilder`] for a [`Query`] that accepts a callback to configure the [`QueryBuilder`].
    /// This boxes the callback so that it has a common type and can be put in a `Vec`.
    pub fn new_box(f: impl FnOnce(&mut QueryBuilder<D, F>) + 'a) -> Self {
        Self(Box::new(f))
    }
}

// SAFETY: Calls `init_query_param`, just like `Query::init_state`.
unsafe impl<
        'w,
        's,
        D: QueryData + 'static,
        F: QueryFilter + 'static,
        T: FnOnce(&mut QueryBuilder<D, F>),
    > SystemParamBuilder<Query<'w, 's, D, F>> for QueryParamBuilder<T>
{
    fn build(self, world: &mut World, system_meta: &mut SystemMeta) -> QueryState<D, F> {
        let mut builder = QueryBuilder::new(world);
        (self.0)(&mut builder);
        let state = builder.build();
        init_query_param(world, system_meta, &state);
        state
    }
}

macro_rules! impl_system_param_builder_tuple {
    ($(($param: ident, $builder: ident)),*) => {
        // SAFETY: implementors of each `SystemParamBuilder` in the tuple have validated their impls
        unsafe impl<$($param: SystemParam,)* $($builder: SystemParamBuilder<$param>,)*> SystemParamBuilder<($($param,)*)> for ($($builder,)*) {
            fn build(self, _world: &mut World, _meta: &mut SystemMeta) -> <($($param,)*) as SystemParam>::State {
                #[allow(non_snake_case)]
                let ($($builder,)*) = self;
                #[allow(clippy::unused_unit)]
                ($($builder.build(_world, _meta),)*)
            }
        }
    };
}

all_tuples!(impl_system_param_builder_tuple, 0, 16, P, B);

/// A [`SystemParamBuilder`] for a [`ParamSet`].
/// To build a [`ParamSet`] with a tuple of system parameters, pass a tuple of matching [`SystemParamBuilder`]s.
/// To build a [`ParamSet`] with a `Vec` of system parameters, pass a `Vec` of matching [`SystemParamBuilder`]s.
pub struct ParamSetBuilder<T>(pub T);

macro_rules! impl_param_set_builder_tuple {
    ($(($param: ident, $builder: ident, $meta: ident)),*) => {
        // SAFETY: implementors of each `SystemParamBuilder` in the tuple have validated their impls
        unsafe impl<'w, 's, $($param: SystemParam,)* $($builder: SystemParamBuilder<$param>,)*> SystemParamBuilder<ParamSet<'w, 's, ($($param,)*)>> for ParamSetBuilder<($($builder,)*)> {
            #[allow(non_snake_case)]
            fn build(self, _world: &mut World, _system_meta: &mut SystemMeta) -> <($($param,)*) as SystemParam>::State {
                let ParamSetBuilder(($($builder,)*)) = self;
                // Note that this is slightly different from `init_state`, which calls `init_state` on each param twice.
                // One call populates an empty `SystemMeta` with the new access, while the other runs against a cloned `SystemMeta` to check for conflicts.
                // Builders can only be invoked once, so we do both in a single call here.
                // That means that any `filtered_accesses` in the `component_access_set` will get copied to every `$meta`
                // and will appear multiple times in the final `SystemMeta`.
                $(
                    let mut $meta = _system_meta.clone();
                    let $param = $builder.build(_world, &mut $meta);
                )*
                // Make the ParamSet non-send if any of its parameters are non-send.
                if false $(|| !$meta.is_send())* {
                    _system_meta.set_non_send();
                }
                $(
                    _system_meta
                        .component_access_set
                        .extend($meta.component_access_set);
                    _system_meta
                        .archetype_component_access
                        .extend(&$meta.archetype_component_access);
                )*
                #[allow(clippy::unused_unit)]
                ($($param,)*)
            }
        }
    };
}

all_tuples!(impl_param_set_builder_tuple, 1, 8, P, B, meta);

/// A [`SystemParamBuilder`] for a [`DynSystemParam`].
pub struct DynParamBuilder<'a>(
    Box<dyn FnOnce(&mut World, &mut SystemMeta) -> DynSystemParamState + 'a>,
);

impl<'a> DynParamBuilder<'a> {
    /// Creates a new [`DynParamBuilder`] by wrapping a [`SystemParamBuilder`] of any type.
    /// The built [`DynSystemParam`] can be downcast to `T`.
    pub fn new<T: SystemParam + 'static>(builder: impl SystemParamBuilder<T> + 'a) -> Self {
        Self(Box::new(|world, meta| {
            DynSystemParamState::new::<T>(builder.build(world, meta))
        }))
    }
}

// SAFETY: `DynSystemParam::get_param` will call `get_param` on the boxed `DynSystemParamState`,
// and the boxed builder was a valid implementation of `SystemParamBuilder` for that type.
// The resulting `DynSystemParam` can only perform access by downcasting to that param type.
unsafe impl<'a, 'w, 's> SystemParamBuilder<DynSystemParam<'w, 's>> for DynParamBuilder<'a> {
    fn build(
        self,
        world: &mut World,
        meta: &mut SystemMeta,
    ) -> <DynSystemParam<'w, 's> as SystemParam>::State {
        (self.0)(world, meta)
    }
}

/// A [`SystemParamBuilder`] for a [`Local`].
/// The provided value will be used as the initial value of the `Local`.
pub struct LocalBuilder<T>(pub T);

// SAFETY: `Local` performs no world access.
unsafe impl<'s, T: FromWorld + Send + 'static> SystemParamBuilder<Local<'s, T>>
    for LocalBuilder<T>
{
    fn build(
        self,
        _world: &mut World,
        _meta: &mut SystemMeta,
    ) -> <Local<'s, T> as SystemParam>::State {
        SyncCell::new(self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::entity::Entities;
    use crate::prelude::{Component, Query};
    use crate::system::{Local, RunSystemOnce};

    use super::*;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

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

        let system = (LocalBuilder(10),)
            .build_state(&mut world)
            .build_system(local_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 10);
    }

    #[test]
    fn query_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (QueryParamBuilder::new(|query| {
            query.with::<A>();
        }),)
            .build_state(&mut world)
            .build_system(query_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }

    #[test]
    fn query_builder_state() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let state = QueryBuilder::new(&mut world).with::<A>().build();

        let system = (state,).build_state(&mut world).build_system(query_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }

    #[test]
    fn multi_param_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (LocalBuilder(0), ParamBuilder)
            .build_state(&mut world)
            .build_system(multi_param_system);

        let result = world.run_system_once(system);
        assert_eq!(result, 1);
    }

    #[test]
    fn param_set_builder() {
        let mut world = World::new();

        world.spawn((A, B, C));
        world.spawn((A, B));
        world.spawn((A, C));
        world.spawn((A, C));
        world.spawn_empty();

        let system = (ParamSetBuilder((
            QueryParamBuilder::new(|builder| {
                builder.with::<B>();
            }),
            QueryParamBuilder::new(|builder| {
                builder.with::<C>();
            }),
        )),)
            .build_state(&mut world)
            .build_system(|mut params: ParamSet<(Query<&mut A>, Query<&mut A>)>| {
                params.p0().iter().count() + params.p1().iter().count()
            });

        let result = world.run_system_once(system);
        assert_eq!(result, 5);
    }

    #[test]
    fn dyn_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (
            DynParamBuilder::new(LocalBuilder(3_usize)),
            DynParamBuilder::new::<Query<()>>(QueryParamBuilder::new(|builder| {
                builder.with::<A>();
            })),
            DynParamBuilder::new::<&Entities>(ParamBuilder),
        )
            .build_state(&mut world)
            .build_system(
                |mut p0: DynSystemParam, mut p1: DynSystemParam, mut p2: DynSystemParam| {
                    let local = *p0.downcast_mut::<Local<usize>>().unwrap();
                    let query_count = p1.downcast_mut::<Query<()>>().unwrap().iter().count();
                    let _entities = p2.downcast_mut::<&Entities>().unwrap();
                    assert!(p0.downcast_mut::<Query<()>>().is_none());
                    local + query_count
                },
            );

        let result = world.run_system_once(system);
        assert_eq!(result, 4);
    }
}
