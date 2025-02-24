use alloc::{boxed::Box, vec::Vec};
use bevy_utils::synccell::SyncCell;
use variadics_please::all_tuples;

use crate::{
    prelude::QueryBuilder,
    query::{QueryData, QueryFilter, QueryState},
    resource::Resource,
    system::{
        DynSystemParam, DynSystemParamState, Local, ParamSet, Query, SystemMeta, SystemParam,
    },
    world::{
        FilteredResources, FilteredResourcesBuilder, FilteredResourcesMut,
        FilteredResourcesMutBuilder, FromWorld, World,
    },
};
use core::fmt::Debug;

use super::{init_query_param, Res, ResMut, SystemState};

/// A builder that can create a [`SystemParam`].
///
/// ```
/// # use bevy_ecs::{
/// #     prelude::*,
/// #     system::{SystemParam, ParamBuilder},
/// # };
/// # #[derive(Resource)]
/// # struct R;
/// #
/// # #[derive(SystemParam)]
/// # struct MyParam;
/// #
/// fn some_system(param: MyParam) {}
///
/// fn build_system(builder: impl SystemParamBuilder<MyParam>) {
///     let mut world = World::new();
///     // To build a system, create a tuple of `SystemParamBuilder`s
///     // with a builder for each parameter.
///     // Note that the builder for a system must be a tuple,
///     // even if there is only one parameter.
///     (builder,)
///         .build_state(&mut world)
///         .build_system(some_system);
/// }
///
/// fn build_closure_system_infer(builder: impl SystemParamBuilder<MyParam>) {
///     let mut world = World::new();
///     // Closures can be used in addition to named functions.
///     // If a closure is used, the parameter types must all be inferred
///     // from the builders, so you cannot use plain `ParamBuilder`.
///     (builder, ParamBuilder::resource())
///         .build_state(&mut world)
///         .build_system(|param, res| {
///             let param: MyParam = param;
///             let res: Res<R> = res;
///         });
/// }
///
/// fn build_closure_system_explicit(builder: impl SystemParamBuilder<MyParam>) {
///     let mut world = World::new();
///     // Alternately, you can provide all types in the closure
///     // parameter list and call `build_any_system()`.
///     (builder, ParamBuilder)
///         .build_state(&mut world)
///         .build_any_system(|param: MyParam, res: Res<R>| {});
/// }
/// ```
///
/// See the documentation for individual builders for more examples.
///
/// # List of Builders
///
/// [`ParamBuilder`] can be used for parameters that don't require any special building.
/// Using a `ParamBuilder` will build the system parameter the same way it would be initialized in an ordinary system.
///
/// `ParamBuilder` also provides factory methods that return a `ParamBuilder` typed as `impl SystemParamBuilder<P>`
/// for common system parameters that can be used to guide closure parameter inference.
///
/// [`QueryParamBuilder`] can build a [`Query`] to add additional filters,
/// or to configure the components available to [`FilteredEntityRef`](crate::world::FilteredEntityRef) or [`FilteredEntityMut`](crate::world::FilteredEntityMut).
/// You can also use a [`QueryState`] to build a [`Query`].
///
/// [`LocalBuilder`] can build a [`Local`] to supply the initial value for the `Local`.
///
/// [`FilteredResourcesParamBuilder`] can build a [`FilteredResources`],
/// and [`FilteredResourcesMutParamBuilder`] can build a [`FilteredResourcesMut`],
/// to configure the resources that can be accessed.
///
/// [`DynParamBuilder`] can build a [`DynSystemParam`] to determine the type of the inner parameter,
/// and to supply any `SystemParamBuilder` it needs.
///
/// Tuples of builders can build tuples of parameters, one builder for each element.
/// Note that since systems require a tuple as a parameter, the outer builder for a system will always be a tuple.
///
/// A [`Vec`] of builders can build a `Vec` of parameters, one builder for each element.
///
/// A [`ParamSetBuilder`] can build a [`ParamSet`].
/// This can wrap either a tuple or a `Vec`, one builder for each element.
///
/// A custom system param created with `#[derive(SystemParam)]` can be buildable if it includes a `#[system_param(builder)]` attribute.
/// See [the documentation for `SystemParam` derives](SystemParam#builders).
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
///
/// ## Example
///
/// ```
/// # use bevy_ecs::{
/// #     prelude::*,
/// #     system::{SystemParam, ParamBuilder},
/// # };
/// #
/// # #[derive(Component)]
/// # struct A;
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
/// fn my_system(res: Res<R>, param: MyParam, query: Query<&A>) {
///     // ...
/// }
///
/// let system = (
///     // A plain ParamBuilder can build any parameter type.
///     ParamBuilder,
///     // The `of::<P>()` method returns a `ParamBuilder`
///     // typed as `impl SystemParamBuilder<P>`.
///     ParamBuilder::of::<MyParam>(),
///     // The other factory methods return typed builders
///     // for common parameter types.
///     ParamBuilder::query::<&A>(),
/// )
///     .build_state(&mut world)
///     .build_system(my_system);
/// ```
#[derive(Default, Debug, Clone)]
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
/// This takes a closure accepting an `&mut` [`QueryBuilder`] and uses the builder to construct the query's state.
/// This can be used to add additional filters,
/// or to configure the components available to [`FilteredEntityRef`](crate::world::FilteredEntityRef) or [`FilteredEntityMut`](crate::world::FilteredEntityMut).
///
/// ## Example
///
/// ```
/// # use bevy_ecs::{
/// #     prelude::*,
/// #     system::{SystemParam, QueryParamBuilder},
/// # };
/// #
/// # #[derive(Component)]
/// # struct Player;
/// #
/// # let mut world = World::new();
/// let system = (QueryParamBuilder::new(|builder| {
///     builder.with::<Player>();
/// }),)
///     .build_state(&mut world)
///     .build_system(|query: Query<()>| {
///         for _ in &query {
///             // This only includes entities with a `Player` component.
///         }
///     });
///
/// // When collecting multiple builders into a `Vec`,
/// // use `new_box()` to erase the closure type.
/// let system = (vec![
///     QueryParamBuilder::new_box(|builder| {
///         builder.with::<Player>();
///     }),
///     QueryParamBuilder::new_box(|builder| {
///         builder.without::<Player>();
///     }),
/// ],)
///     .build_state(&mut world)
///     .build_system(|query: Vec<Query<()>>| {});
/// ```
#[derive(Clone)]
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
    ($(#[$meta:meta])* $(($param: ident, $builder: ident)),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is in a macro; as such, the below lints may not always apply."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        #[allow(
            non_snake_case,
            reason = "The variable names are provided by the macro caller, not by us."
        )]
        $(#[$meta])*
        // SAFETY: implementors of each `SystemParamBuilder` in the tuple have validated their impls
        unsafe impl<$($param: SystemParam,)* $($builder: SystemParamBuilder<$param>,)*> SystemParamBuilder<($($param,)*)> for ($($builder,)*) {
            fn build(self, world: &mut World, meta: &mut SystemMeta) -> <($($param,)*) as SystemParam>::State {
                let ($($builder,)*) = self;
                #[allow(
                    clippy::unused_unit,
                    reason = "Zero-length tuples won't generate any calls to the system parameter builders."
                )]
                ($($builder.build(world, meta),)*)
            }
        }
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_system_param_builder_tuple,
    0,
    16,
    P,
    B
);

// SAFETY: implementors of each `SystemParamBuilder` in the vec have validated their impls
unsafe impl<P: SystemParam, B: SystemParamBuilder<P>> SystemParamBuilder<Vec<P>> for Vec<B> {
    fn build(self, world: &mut World, meta: &mut SystemMeta) -> <Vec<P> as SystemParam>::State {
        self.into_iter()
            .map(|builder| builder.build(world, meta))
            .collect()
    }
}

/// A [`SystemParamBuilder`] for a [`ParamSet`].
///
/// To build a [`ParamSet`] with a tuple of system parameters, pass a tuple of matching [`SystemParamBuilder`]s.
/// To build a [`ParamSet`] with a [`Vec`] of system parameters, pass a `Vec` of matching [`SystemParamBuilder`]s.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Component)]
/// # struct Health;
/// #
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// # #[derive(Component)]
/// # struct Ally;
/// #
/// # let mut world = World::new();
/// #
/// let system = (ParamSetBuilder((
///     QueryParamBuilder::new(|builder| {
///         builder.with::<Enemy>();
///     }),
///     QueryParamBuilder::new(|builder| {
///         builder.with::<Ally>();
///     }),
///     ParamBuilder,
/// )),)
///     .build_state(&mut world)
///     .build_system(buildable_system_with_tuple);
/// # world.run_system_once(system);
///
/// fn buildable_system_with_tuple(
///     mut set: ParamSet<(Query<&mut Health>, Query<&mut Health>, &World)>,
/// ) {
///     // The first parameter is built from the first builder,
///     // so this will iterate over enemies.
///     for mut health in set.p0().iter_mut() {}
///     // And the second parameter is built from the second builder,
///     // so this will iterate over allies.
///     for mut health in set.p1().iter_mut() {}
///     // Parameters that don't need special building can use `ParamBuilder`.
///     let entities = set.p2().entities();
/// }
///
/// let system = (ParamSetBuilder(vec![
///     QueryParamBuilder::new_box(|builder| {
///         builder.with::<Enemy>();
///     }),
///     QueryParamBuilder::new_box(|builder| {
///         builder.with::<Ally>();
///     }),
/// ]),)
///     .build_state(&mut world)
///     .build_system(buildable_system_with_vec);
/// # world.run_system_once(system);
///
/// fn buildable_system_with_vec(mut set: ParamSet<Vec<Query<&mut Health>>>) {
///     // As with tuples, the first parameter is built from the first builder,
///     // so this will iterate over enemies.
///     for mut health in set.get_mut(0).iter_mut() {}
///     // And the second parameter is built from the second builder,
///     // so this will iterate over allies.
///     for mut health in set.get_mut(1).iter_mut() {}
///     // You can iterate over the parameters either by index,
///     // or using the `for_each` method.
///     set.for_each(|mut query| for mut health in query.iter_mut() {});
/// }
/// ```
#[derive(Debug, Default, Clone)]
pub struct ParamSetBuilder<T>(pub T);

macro_rules! impl_param_set_builder_tuple {
    ($(($param: ident, $builder: ident, $meta: ident)),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is in a macro; as such, the below lints may not always apply."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        #[allow(
            non_snake_case,
            reason = "The variable names are provided by the macro caller, not by us."
        )]
        // SAFETY: implementors of each `SystemParamBuilder` in the tuple have validated their impls
        unsafe impl<'w, 's, $($param: SystemParam,)* $($builder: SystemParamBuilder<$param>,)*> SystemParamBuilder<ParamSet<'w, 's, ($($param,)*)>> for ParamSetBuilder<($($builder,)*)> {
            fn build(self, world: &mut World, system_meta: &mut SystemMeta) -> <($($param,)*) as SystemParam>::State {
                let ParamSetBuilder(($($builder,)*)) = self;
                // Note that this is slightly different from `init_state`, which calls `init_state` on each param twice.
                // One call populates an empty `SystemMeta` with the new access, while the other runs against a cloned `SystemMeta` to check for conflicts.
                // Builders can only be invoked once, so we do both in a single call here.
                // That means that any `filtered_accesses` in the `component_access_set` will get copied to every `$meta`
                // and will appear multiple times in the final `SystemMeta`.
                $(
                    let mut $meta = system_meta.clone();
                    let $param = $builder.build(world, &mut $meta);
                )*
                // Make the ParamSet non-send if any of its parameters are non-send.
                if false $(|| !$meta.is_send())* {
                    system_meta.set_non_send();
                }
                $(
                    system_meta
                        .component_access_set
                        .extend($meta.component_access_set);
                    system_meta
                        .archetype_component_access
                        .extend(&$meta.archetype_component_access);
                )*
                #[allow(
                    clippy::unused_unit,
                    reason = "Zero-length tuples won't generate any calls to the system parameter builders."
                )]
                ($($param,)*)
            }
        }
    };
}

all_tuples!(impl_param_set_builder_tuple, 1, 8, P, B, meta);

// SAFETY: Relevant parameter ComponentId and ArchetypeComponentId access is applied to SystemMeta. If any ParamState conflicts
// with any prior access, a panic will occur.
unsafe impl<'w, 's, P: SystemParam, B: SystemParamBuilder<P>>
    SystemParamBuilder<ParamSet<'w, 's, Vec<P>>> for ParamSetBuilder<Vec<B>>
{
    fn build(
        self,
        world: &mut World,
        system_meta: &mut SystemMeta,
    ) -> <Vec<P> as SystemParam>::State {
        let mut states = Vec::with_capacity(self.0.len());
        let mut metas = Vec::with_capacity(self.0.len());
        for builder in self.0 {
            let mut meta = system_meta.clone();
            states.push(builder.build(world, &mut meta));
            metas.push(meta);
        }
        if metas.iter().any(|m| !m.is_send()) {
            system_meta.set_non_send();
        }
        for meta in metas {
            system_meta
                .component_access_set
                .extend(meta.component_access_set);
            system_meta
                .archetype_component_access
                .extend(&meta.archetype_component_access);
        }
        states
    }
}

/// A [`SystemParamBuilder`] for a [`DynSystemParam`].
/// See the [`DynSystemParam`] docs for examples.
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
///
/// ## Example
///
/// ```
/// # use bevy_ecs::{
/// #     prelude::*,
/// #     system::{SystemParam, LocalBuilder, RunSystemOnce},
/// # };
/// #
/// # let mut world = World::new();
/// let system = (LocalBuilder(100),)
///     .build_state(&mut world)
///     .build_system(|local: Local<usize>| {
///         assert_eq!(*local, 100);
///     });
/// # world.run_system_once(system);
/// ```
#[derive(Default, Debug, Clone)]
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

/// A [`SystemParamBuilder`] for a [`FilteredResources`].
/// See the [`FilteredResources`] docs for examples.
#[derive(Clone)]
pub struct FilteredResourcesParamBuilder<T>(T);

impl<T> FilteredResourcesParamBuilder<T> {
    /// Creates a [`SystemParamBuilder`] for a [`FilteredResources`] that accepts a callback to configure the [`FilteredResourcesBuilder`].
    pub fn new(f: T) -> Self
    where
        T: FnOnce(&mut FilteredResourcesBuilder),
    {
        Self(f)
    }
}

impl<'a> FilteredResourcesParamBuilder<Box<dyn FnOnce(&mut FilteredResourcesBuilder) + 'a>> {
    /// Creates a [`SystemParamBuilder`] for a [`FilteredResources`] that accepts a callback to configure the [`FilteredResourcesBuilder`].
    /// This boxes the callback so that it has a common type.
    pub fn new_box(f: impl FnOnce(&mut FilteredResourcesBuilder) + 'a) -> Self {
        Self(Box::new(f))
    }
}

// SAFETY: Resource ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this FilteredResources
// conflicts with any prior access, a panic will occur.
unsafe impl<'w, 's, T: FnOnce(&mut FilteredResourcesBuilder)>
    SystemParamBuilder<FilteredResources<'w, 's>> for FilteredResourcesParamBuilder<T>
{
    fn build(
        self,
        world: &mut World,
        meta: &mut SystemMeta,
    ) -> <FilteredResources<'w, 's> as SystemParam>::State {
        let mut builder = FilteredResourcesBuilder::new(world);
        (self.0)(&mut builder);
        let access = builder.build();

        let combined_access = meta.component_access_set.combined_access();
        let conflicts = combined_access.get_conflicts(&access);
        if !conflicts.is_empty() {
            let accesses = conflicts.format_conflict_list(world);
            let system_name = &meta.name;
            panic!("error[B0002]: FilteredResources in system {system_name} accesses resources(s){accesses} in a way that conflicts with a previous system parameter. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002");
        }

        if access.has_read_all_resources() {
            meta.component_access_set
                .add_unfiltered_read_all_resources();
            meta.archetype_component_access.read_all_resources();
        } else {
            for component_id in access.resource_reads_and_writes() {
                meta.component_access_set
                    .add_unfiltered_resource_read(component_id);

                let archetype_component_id = world.initialize_resource_internal(component_id).id();
                meta.archetype_component_access
                    .add_resource_read(archetype_component_id);
            }
        }

        access
    }
}

/// A [`SystemParamBuilder`] for a [`FilteredResourcesMut`].
/// See the [`FilteredResourcesMut`] docs for examples.
#[derive(Clone)]
pub struct FilteredResourcesMutParamBuilder<T>(T);

impl<T> FilteredResourcesMutParamBuilder<T> {
    /// Creates a [`SystemParamBuilder`] for a [`FilteredResourcesMut`] that accepts a callback to configure the [`FilteredResourcesMutBuilder`].
    pub fn new(f: T) -> Self
    where
        T: FnOnce(&mut FilteredResourcesMutBuilder),
    {
        Self(f)
    }
}

impl<'a> FilteredResourcesMutParamBuilder<Box<dyn FnOnce(&mut FilteredResourcesMutBuilder) + 'a>> {
    /// Creates a [`SystemParamBuilder`] for a [`FilteredResourcesMut`] that accepts a callback to configure the [`FilteredResourcesMutBuilder`].
    /// This boxes the callback so that it has a common type.
    pub fn new_box(f: impl FnOnce(&mut FilteredResourcesMutBuilder) + 'a) -> Self {
        Self(Box::new(f))
    }
}

// SAFETY: Resource ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this FilteredResources
// conflicts with any prior access, a panic will occur.
unsafe impl<'w, 's, T: FnOnce(&mut FilteredResourcesMutBuilder)>
    SystemParamBuilder<FilteredResourcesMut<'w, 's>> for FilteredResourcesMutParamBuilder<T>
{
    fn build(
        self,
        world: &mut World,
        meta: &mut SystemMeta,
    ) -> <FilteredResourcesMut<'w, 's> as SystemParam>::State {
        let mut builder = FilteredResourcesMutBuilder::new(world);
        (self.0)(&mut builder);
        let access = builder.build();

        let combined_access = meta.component_access_set.combined_access();
        let conflicts = combined_access.get_conflicts(&access);
        if !conflicts.is_empty() {
            let accesses = conflicts.format_conflict_list(world);
            let system_name = &meta.name;
            panic!("error[B0002]: FilteredResourcesMut in system {system_name} accesses resources(s){accesses} in a way that conflicts with a previous system parameter. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002");
        }

        if access.has_read_all_resources() {
            meta.component_access_set
                .add_unfiltered_read_all_resources();
            meta.archetype_component_access.read_all_resources();
        } else {
            for component_id in access.resource_reads() {
                meta.component_access_set
                    .add_unfiltered_resource_read(component_id);

                let archetype_component_id = world.initialize_resource_internal(component_id).id();
                meta.archetype_component_access
                    .add_resource_read(archetype_component_id);
            }
        }

        if access.has_write_all_resources() {
            meta.component_access_set
                .add_unfiltered_write_all_resources();
            meta.archetype_component_access.write_all_resources();
        } else {
            for component_id in access.resource_writes() {
                meta.component_access_set
                    .add_unfiltered_resource_write(component_id);

                let archetype_component_id = world.initialize_resource_internal(component_id).id();
                meta.archetype_component_access
                    .add_resource_write(archetype_component_id);
            }
        }

        access
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entities,
        prelude::{Component, Query},
        reflect::ReflectResource,
        system::{Local, RunSystemOnce},
    };
    use alloc::vec;
    use bevy_reflect::{FromType, Reflect, ReflectRef};

    use super::*;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(Resource, Default, Reflect)]
    #[reflect(Resource)]
    struct R {
        foo: usize,
    }

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

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 10);
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

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 1);
    }

    #[test]
    fn query_builder_state() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let state = QueryBuilder::new(&mut world).with::<A>().build();

        let system = (state,).build_state(&mut world).build_system(query_system);

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 1);
    }

    #[test]
    fn multi_param_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (LocalBuilder(0), ParamBuilder)
            .build_state(&mut world)
            .build_system(multi_param_system);

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 1);
    }

    #[test]
    fn vec_builder() {
        let mut world = World::new();

        world.spawn((A, B, C));
        world.spawn((A, B));
        world.spawn((A, C));
        world.spawn((A, C));
        world.spawn_empty();

        let system = (vec![
            QueryParamBuilder::new_box(|builder| {
                builder.with::<B>().without::<C>();
            }),
            QueryParamBuilder::new_box(|builder| {
                builder.with::<C>().without::<B>();
            }),
        ],)
            .build_state(&mut world)
            .build_system(|params: Vec<Query<&mut A>>| {
                let mut count: usize = 0;
                params
                    .into_iter()
                    .for_each(|mut query| count += query.iter_mut().count());
                count
            });

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 3);
    }

    #[test]
    fn multi_param_builder_inference() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (LocalBuilder(0u64), ParamBuilder::local::<u64>())
            .build_state(&mut world)
            .build_system(|a, b| *a + *b + 1);

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 1);
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

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 5);
    }

    #[test]
    fn param_set_vec_builder() {
        let mut world = World::new();

        world.spawn((A, B, C));
        world.spawn((A, B));
        world.spawn((A, C));
        world.spawn((A, C));
        world.spawn_empty();

        let system = (ParamSetBuilder(vec![
            QueryParamBuilder::new_box(|builder| {
                builder.with::<B>();
            }),
            QueryParamBuilder::new_box(|builder| {
                builder.with::<C>();
            }),
        ]),)
            .build_state(&mut world)
            .build_system(|mut params: ParamSet<Vec<Query<&mut A>>>| {
                let mut count = 0;
                params.for_each(|mut query| count += query.iter_mut().count());
                count
            });

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 5);
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

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 4);
    }

    #[derive(SystemParam)]
    #[system_param(builder)]
    struct CustomParam<'w, 's> {
        query: Query<'w, 's, ()>,
        local: Local<'s, usize>,
    }

    #[test]
    fn custom_param_builder() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (CustomParamBuilder {
            local: LocalBuilder(100),
            query: QueryParamBuilder::new(|builder| {
                builder.with::<A>();
            }),
        },)
            .build_state(&mut world)
            .build_system(|param: CustomParam| *param.local + param.query.iter().count());

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 101);
    }

    #[test]
    fn filtered_resource_conflicts_read_with_res() {
        let mut world = World::new();
        (
            ParamBuilder::resource(),
            FilteredResourcesParamBuilder::new(|builder| {
                builder.add_read::<R>();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: Res<R>, _fr: FilteredResources| {});
    }

    #[test]
    #[should_panic]
    fn filtered_resource_conflicts_read_with_resmut() {
        let mut world = World::new();
        (
            ParamBuilder::resource_mut(),
            FilteredResourcesParamBuilder::new(|builder| {
                builder.add_read::<R>();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: ResMut<R>, _fr: FilteredResources| {});
    }

    #[test]
    #[should_panic]
    fn filtered_resource_conflicts_read_all_with_resmut() {
        let mut world = World::new();
        (
            ParamBuilder::resource_mut(),
            FilteredResourcesParamBuilder::new(|builder| {
                builder.add_read_all();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: ResMut<R>, _fr: FilteredResources| {});
    }

    #[test]
    fn filtered_resource_mut_conflicts_read_with_res() {
        let mut world = World::new();
        (
            ParamBuilder::resource(),
            FilteredResourcesMutParamBuilder::new(|builder| {
                builder.add_read::<R>();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: Res<R>, _fr: FilteredResourcesMut| {});
    }

    #[test]
    #[should_panic]
    fn filtered_resource_mut_conflicts_read_with_resmut() {
        let mut world = World::new();
        (
            ParamBuilder::resource_mut(),
            FilteredResourcesMutParamBuilder::new(|builder| {
                builder.add_read::<R>();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: ResMut<R>, _fr: FilteredResourcesMut| {});
    }

    #[test]
    #[should_panic]
    fn filtered_resource_mut_conflicts_write_with_res() {
        let mut world = World::new();
        (
            ParamBuilder::resource(),
            FilteredResourcesMutParamBuilder::new(|builder| {
                builder.add_write::<R>();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: Res<R>, _fr: FilteredResourcesMut| {});
    }

    #[test]
    #[should_panic]
    fn filtered_resource_mut_conflicts_write_all_with_res() {
        let mut world = World::new();
        (
            ParamBuilder::resource(),
            FilteredResourcesMutParamBuilder::new(|builder| {
                builder.add_write_all();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: Res<R>, _fr: FilteredResourcesMut| {});
    }

    #[test]
    #[should_panic]
    fn filtered_resource_mut_conflicts_write_with_resmut() {
        let mut world = World::new();
        (
            ParamBuilder::resource_mut(),
            FilteredResourcesMutParamBuilder::new(|builder| {
                builder.add_write::<R>();
            }),
        )
            .build_state(&mut world)
            .build_system(|_r: ResMut<R>, _fr: FilteredResourcesMut| {});
    }

    #[test]
    fn filtered_resource_reflect() {
        let mut world = World::new();
        world.insert_resource(R { foo: 7 });

        let system = (FilteredResourcesParamBuilder::new(|builder| {
            builder.add_read::<R>();
        }),)
            .build_state(&mut world)
            .build_system(|res: FilteredResources| {
                let reflect_resource = <ReflectResource as FromType<R>>::from_type();
                let ReflectRef::Struct(reflect_struct) =
                    reflect_resource.reflect(res).unwrap().reflect_ref()
                else {
                    panic!()
                };
                *reflect_struct
                    .field("foo")
                    .unwrap()
                    .try_downcast_ref::<usize>()
                    .unwrap()
            });

        let output = world.run_system_once(system).unwrap();
        assert_eq!(output, 7);
    }
}
