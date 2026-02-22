use alloc::{boxed::Box, vec::Vec};
use bevy_platform::cell::SyncCell;
use bevy_utils::prelude::DebugName;
use variadics_please::all_tuples;

use crate::{
    change_detection::{CheckChangeTicks, Tick},
    prelude::QueryBuilder,
    query::{FilteredAccessSet, QueryData, QueryFilter, QueryState},
    resource::Resource,
    system::{
        DynSystemParam, DynSystemParamState, FromInput, FunctionSystem, If, IntoResult, IntoSystem,
        Local, ParamSet, Query, ReadOnlySystem, System, SystemInput, SystemMeta, SystemParam,
        SystemParamFunction, SystemParamValidationError,
    },
    world::{
        unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FilteredResources,
        FilteredResourcesBuilder, FilteredResourcesMut, FilteredResourcesMutBuilder, FromWorld,
        World,
    },
};
use core::{fmt::Debug, marker::PhantomData, mem};

use super::{Res, ResMut, RunSystemError, SharedStates, SystemState, SystemStateFlags};

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
/// fn build_system(builder: impl SystemParamBuilder<MyParam> + 'static) {
///     // To build a system, create a tuple of `SystemParamBuilder`s
///     // with a builder for each parameter.
///     // Note that the builder for a system must be a tuple,
///     // even if there is only one parameter.
/// #   let _system: bevy_ecs::system::IntoBuilderSystem<fn(MyParam), (), (), _, _> =
///     (builder,)
///         .build_system(some_system);
/// }
///
/// fn build_system_direct(builder: impl SystemParamBuilder<MyParam>) {
///     let mut world = World::new();
///     // You can also construct a system in two steps, first by
///     // constructing a [`SystemState`] with `build_state` and
///     // second by constructing the final system with `build_system`.
///     // This can be useful in cases that require type inference
///     // for function parameters (like closures!), since normal
///     // `build_system` requires explicitly specifying all parameter
///     // types. See `build_closure_system_infer/explicit` below for more
///     // info.
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
///     // parameter list and call `build_system()` normally.
///     (builder, ParamBuilder::resource())
///         .build_state(&mut world) // this line can be optionally omitted, since all the parameter types are explicit!
///         .build_system(|param: MyParam, res: Res<R>| {});
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
/// The implementor must ensure that the state returned
/// from [`SystemParamBuilder::build`] is valid for `P`.
/// Note that the exact safety requirements depend on the implementation of [`SystemParam`],
/// so if `Self` is not a local type then you must call [`SystemParam::init_state`]
/// or another [`SystemParamBuilder::build`].
pub unsafe trait SystemParamBuilder<P: SystemParam>: Sized {
    /// Registers any [`World`] access used by this [`SystemParam`]
    /// and creates a new instance of this param's [`State`](SystemParam::State).
    ///
    /// # Safety
    /// The new state must not outlive `SharedStates`
    unsafe fn build(self, world: &mut World, shared_states: &SharedStates) -> P::State;

    /// Create a [`SystemState`] from a [`SystemParamBuilder`].
    /// To create a system, call [`SystemState::build_system`] on the result.
    fn build_state(self, world: &mut World) -> SystemState<P> {
        SystemState::from_builder(world, self)
    }

    /// Create a [`System`] from a [`SystemParamBuilder`] directly.
    ///
    /// This method is useful in cases where type inference for
    /// closure parameters isn't necessary, or where it's not
    /// possible to call [`SystemState::build_system`] by passing
    /// in an `&mut World`. Rather than constructing the system's
    /// state immediately, this function returns a wrapper that
    /// initializes the system state during the first run.
    ///
    /// Caveats:
    /// - doesn't support parameter type inference.
    /// - only works for 'static system param builder types.
    ///
    /// In cases where  either of these are required, call
    /// [`SystemParamBuilder::build_state`] instead.
    fn build_system<Marker, In, Out, Func>(
        self,
        func: Func,
    ) -> IntoBuilderSystem<Marker, In, Out, Func, Self>
    where
        Self: 'static,
        Func: SystemParamFunction<Marker, Param = P>,
    {
        IntoBuilderSystem::new(self, func)
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
    unsafe fn build(self, world: &mut World, shared_states: &SharedStates) -> P::State {
        // SAFETY: requirements are upheld by caller
        unsafe { P::init_state(world, shared_states) }
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

/// A marker type used to distinguish builder systems from plain function systems.
#[doc(hidden)]
pub struct IsBuilderSystem;

/// An [`IntoSystem`] creating an instance of [`BuilderSystem`]
pub struct IntoBuilderSystem<Marker, In, Out, Func, Builder>
where
    Func: SystemParamFunction<Marker>,
    Builder: SystemParamBuilder<Func::Param>,
{
    builder: Builder,
    func: Func,
    _marker: PhantomData<fn(In) -> (Marker, Out)>,
}

impl<Marker, In, Out, Func, Builder> IntoBuilderSystem<Marker, In, Out, Func, Builder>
where
    Func: SystemParamFunction<Marker>,
    Builder: SystemParamBuilder<Func::Param>,
{
    /// Returns a new [`IntoBuilderSystem`] given a system param builder and system function
    pub fn new(builder: Builder, func: Func) -> Self {
        Self {
            builder,
            func,
            _marker: PhantomData,
        }
    }
}

impl<Marker, In, Out, Func, Builder> IntoSystem<In, Out, (IsBuilderSystem, Marker)>
    for IntoBuilderSystem<Marker, In, Out, Func, Builder>
where
    Marker: 'static,
    In: SystemInput + 'static,
    Out: 'static,
    Func: SystemParamFunction<Marker, In: FromInput<In>, Out: IntoResult<Out>>,
    Builder: SystemParamBuilder<Func::Param> + Send + Sync + 'static,
{
    type System = BuilderSystem<Marker, In, Out, Func, Builder>;

    fn into_system(this: Self) -> Self::System {
        BuilderSystem::new(this.builder, this.func)
    }
}

/// A [`System`] created from a [`SystemParamBuilder`] whose state is not
/// initialized until the first run.
pub struct BuilderSystem<Marker, In, Out, Func, Builder>
where
    Func: SystemParamFunction<Marker>,
    Builder: SystemParamBuilder<Func::Param>,
{
    inner: BuilderSystemInner<Marker, In, Out, Func, Builder>,
}

impl<Marker, In, Out, Func, Builder> BuilderSystem<Marker, In, Out, Func, Builder>
where
    Func: SystemParamFunction<Marker>,
    Builder: SystemParamBuilder<Func::Param>,
{
    /// Returns a new `BuilderSystem` given a system param builder and a system function
    pub fn new(builder: Builder, func: Func) -> Self {
        Self {
            inner: BuilderSystemInner::Uninitialized {
                builder,
                func,
                meta: SystemMeta::new::<Func>(),
            },
        }
    }
}

enum BuilderSystemInner<Marker, In, Out, Func, Builder>
where
    Func: SystemParamFunction<Marker>,
    Builder: SystemParamBuilder<Func::Param>,
{
    /// A properly initialized system whose state has been constructed
    Initialized {
        system: FunctionSystem<Marker, In, Out, Func>,
    },
    /// An uninitialized system, whose state hasn't been constructed from
    /// the param builder yet
    Uninitialized {
        builder: Builder,
        func: Func,
        meta: SystemMeta,
    },
    /// This only exists as a variant to use with `mem::replace` in `initialize`.
    /// If this state is ever observed outside `initialize`, then a `panic!`
    /// interrupted initialization, leaving this system in an invalid state.
    Invalid,
}

impl<Marker, In, Out, Func, Builder> System for BuilderSystem<Marker, In, Out, Func, Builder>
where
    Marker: 'static,
    In: SystemInput + 'static,
    Out: 'static,
    Func: SystemParamFunction<Marker, In: FromInput<In>, Out: IntoResult<Out>>,
    Builder: SystemParamBuilder<Func::Param> + Send + Sync + 'static,
{
    type In = In;

    type Out = Out;

    #[inline]
    fn name(&self) -> DebugName {
        match &self.inner {
            BuilderSystemInner::Initialized { system } => system.name(),
            BuilderSystemInner::Uninitialized { meta, .. } => meta.name().clone(),
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        match &self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.flags(),
            BuilderSystemInner::Uninitialized { meta, .. } => meta.flags(),
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: super::SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        match &mut self.inner {
            // SAFETY: requirements upheld by the caller.
            BuilderSystemInner::Initialized { system, .. } => unsafe {
                system.run_unsafe(input, world)
            },
            BuilderSystemInner::Uninitialized { .. } => panic!(
                "BuilderSystem {} was not initialized before calling run_unsafe.",
                self.name()
            ),
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        match &mut self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.refresh_hotpatch(),
            BuilderSystemInner::Uninitialized { .. } => {}
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        match &mut self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.apply_deferred(world),
            BuilderSystemInner::Uninitialized { .. } => {}
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        match &mut self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.queue_deferred(world),
            BuilderSystemInner::Uninitialized { .. } => {}
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        match &mut self.inner {
            // SAFETY: requirements upheld by the caller.
            BuilderSystemInner::Initialized { system, .. } => unsafe {
                system.validate_param_unsafe(world)
            },
            BuilderSystemInner::Uninitialized { .. } => panic!(
                "BuilderSystem {} was not initialized before calling validate_param_unsafe.",
                self.name()
            ),
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        let inner = mem::replace(&mut self.inner, BuilderSystemInner::Invalid);
        match inner {
            BuilderSystemInner::Initialized { mut system } => {
                let access = system.initialize(world);
                self.inner = BuilderSystemInner::Initialized { system };
                access
            }
            BuilderSystemInner::Uninitialized { builder, func, .. } => {
                let mut system = builder.build_state(world).build_any_system(func);
                let access = system.initialize(world);
                self.inner = BuilderSystemInner::Initialized { system };
                access
            }
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        match &mut self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.check_change_tick(check),
            BuilderSystemInner::Uninitialized { .. } => {}
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn get_last_run(&self) -> Tick {
        match &self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.get_last_run(),
            BuilderSystemInner::Uninitialized { meta, .. } => meta.get_last_run(),
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }

    #[inline]
    fn set_last_run(&mut self, last_run: Tick) {
        match &mut self.inner {
            BuilderSystemInner::Initialized { system, .. } => system.set_last_run(last_run),
            BuilderSystemInner::Uninitialized { meta, .. } => meta.set_last_run(last_run),
            BuilderSystemInner::Invalid => unreachable!(),
        }
    }
}

// SAFETY: if the wrapped system is read-only, so is this one
unsafe impl<Marker, In, Out, Func, Builder> ReadOnlySystem
    for BuilderSystem<Marker, In, Out, Func, Builder>
where
    Marker: 'static,
    In: SystemInput + 'static,
    Out: 'static,
    Func: SystemParamFunction<Marker, In: FromInput<In>, Out: IntoResult<Out>>,
    Builder: SystemParamBuilder<Func::Param> + Send + Sync + 'static,
    // the important bound
    FunctionSystem<Marker, In, Out, Func>: ReadOnlySystem,
{
}

// SAFETY: Any `QueryState<D, F>` for the correct world is valid for `Query::State`,
// and we check the world during `build`.
unsafe impl<'w, 's, D: QueryData + 'static, F: QueryFilter + 'static>
    SystemParamBuilder<Query<'w, 's, D, F>> for QueryState<D, F>
{
    // SAFETY: no access to `SharesStates`
    unsafe fn build(self, world: &mut World, _shared_states: &SharedStates) -> QueryState<D, F> {
        self.validate_world(world.id());
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

// SAFETY: Any `QueryState<D, F>` for the correct world is valid for `Query::State`,
// and `QueryBuilder` produces one with the given `world`.
unsafe impl<
        'w,
        's,
        D: QueryData + 'static,
        F: QueryFilter + 'static,
        T: FnOnce(&mut QueryBuilder<D, F>),
    > SystemParamBuilder<Query<'w, 's, D, F>> for QueryParamBuilder<T>
{
    // SAFETY: no access to `SharesStates`
    unsafe fn build(self, world: &mut World, _shared_states: &SharedStates) -> QueryState<D, F> {
        let mut builder = QueryBuilder::new(world);
        (self.0)(&mut builder);
        builder.build()
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
            unsafe fn build(self, world: &mut World, shared_states: &SharedStates) -> <($($param,)*) as SystemParam>::State {
                let ($($builder,)*) = self;
                #[allow(
                    clippy::unused_unit,
                    unused_unsafe,
                    reason = "Zero-length tuples won't generate any calls to the system parameter builders."
                )]
                // SAFETY: requirements are upheld by caller
                unsafe { ($($builder.build(world, shared_states),)*) }
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
    unsafe fn build(
        self,
        world: &mut World,
        shared_states: &SharedStates,
    ) -> <Vec<P> as SystemParam>::State {
        // SAFETY: requirements are upheld by the caller
        self.into_iter()
            .map(|builder| unsafe { builder.build(world, shared_states) })
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
    ($(($param: ident, $builder: ident)),*) => {
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
            unsafe fn build(self, world: &mut World, shared_states: &SharedStates) -> <($($param,)*) as SystemParam>::State {
                let ParamSetBuilder(($($builder,)*)) = self;
                // SAFETY: requirements are upheld by caller
                unsafe { ($($builder.build(world, shared_states),)*) }
            }
        }
    };
}

all_tuples!(impl_param_set_builder_tuple, 1, 8, P, B);

// SAFETY: implementors of each `SystemParamBuilder` in the vec have validated their impls
unsafe impl<'w, 's, P: SystemParam, B: SystemParamBuilder<P>>
    SystemParamBuilder<ParamSet<'w, 's, Vec<P>>> for ParamSetBuilder<Vec<B>>
{
    unsafe fn build(
        self,
        world: &mut World,
        shared_states: &SharedStates,
    ) -> <Vec<P> as SystemParam>::State {
        // SAFETY: requirements are upheld by the caller
        self.0
            .into_iter()
            .map(|builder| unsafe { builder.build(world, shared_states) })
            .collect()
    }
}

/// A [`SystemParamBuilder`] for a [`DynSystemParam`].
/// See the [`DynSystemParam`] docs for examples.
pub struct DynParamBuilder<'a>(
    Box<dyn FnOnce(&mut World, &SharedStates) -> DynSystemParamState + 'a>,
);

impl<'a> DynParamBuilder<'a> {
    /// Creates a new [`DynParamBuilder`] by wrapping a [`SystemParamBuilder`] of any type.
    /// The built [`DynSystemParam`] can be downcast to `T`.
    pub fn new<T: SystemParam + 'static>(builder: impl SystemParamBuilder<T> + 'a) -> Self {
        Self(Box::new(|world, shared_states| {
            // SAFETY: requirements are upheld at call site
            DynSystemParamState::new::<T>(unsafe { builder.build(world, shared_states) })
        }))
    }
}

// SAFETY: `DynSystemParam::get_param` will call `get_param` on the boxed `DynSystemParamState`,
// and the boxed builder was a valid implementation of `SystemParamBuilder` for that type.
// The resulting `DynSystemParam` can only perform access by downcasting to that param type.
unsafe impl<'a, 'w, 's> SystemParamBuilder<DynSystemParam<'w, 's>> for DynParamBuilder<'a> {
    unsafe fn build(
        self,
        world: &mut World,
        shared_states: &SharedStates,
    ) -> <DynSystemParam<'w, 's> as SystemParam>::State {
        (self.0)(world, shared_states)
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

// SAFETY: Any value of `T` is a valid state for `Local`.
unsafe impl<'s, T: FromWorld + Send + 'static> SystemParamBuilder<Local<'s, T>>
    for LocalBuilder<T>
{
    unsafe fn build(
        self,
        _world: &mut World,
        _shared_states: &SharedStates,
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

// SAFETY: Any `Access` is a valid state for `FilteredResources`.
unsafe impl<'w, 's, T: FnOnce(&mut FilteredResourcesBuilder)>
    SystemParamBuilder<FilteredResources<'w, 's>> for FilteredResourcesParamBuilder<T>
{
    unsafe fn build(
        self,
        world: &mut World,
        _shared_states: &SharedStates,
    ) -> <FilteredResources<'w, 's> as SystemParam>::State {
        let mut builder = FilteredResourcesBuilder::new(world);
        (self.0)(&mut builder);
        builder.build()
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

// SAFETY: Any `Access` is a valid state for `FilteredResourcesMut`.
unsafe impl<'w, 's, T: FnOnce(&mut FilteredResourcesMutBuilder)>
    SystemParamBuilder<FilteredResourcesMut<'w, 's>> for FilteredResourcesMutParamBuilder<T>
{
    unsafe fn build(
        self,
        world: &mut World,
        _shared_states: &SharedStates,
    ) -> <FilteredResourcesMut<'w, 's> as SystemParam>::State {
        let mut builder = FilteredResourcesMutBuilder::new(world);
        (self.0)(&mut builder);
        builder.build()
    }
}

/// A [`SystemParamBuilder`] for an [`Option`].
#[derive(Clone)]
pub struct OptionBuilder<T>(T);

// SAFETY: `OptionBuilder<B>` builds a state that is valid for `P`, and any state valid for `P` is valid for `Option<P>`
unsafe impl<P: SystemParam, B: SystemParamBuilder<P>> SystemParamBuilder<Option<P>>
    for OptionBuilder<B>
{
    unsafe fn build(
        self,
        world: &mut World,
        shared_states: &SharedStates,
    ) -> <Option<P> as SystemParam>::State {
        // SAFETY: requirements are upheld by caller
        unsafe { self.0.build(world, shared_states) }
    }
}

/// A [`SystemParamBuilder`] for a [`Result`] of [`SystemParamValidationError`].
#[derive(Clone)]
pub struct ResultBuilder<T>(T);

// SAFETY: `ResultBuilder<B>` builds a state that is valid for `P`, and any state valid for `P` is valid for `Result<P, SystemParamValidationError>`
unsafe impl<P: SystemParam, B: SystemParamBuilder<P>>
    SystemParamBuilder<Result<P, SystemParamValidationError>> for ResultBuilder<B>
{
    unsafe fn build(
        self,
        world: &mut World,
        shared_states: &SharedStates,
    ) -> <Result<P, SystemParamValidationError> as SystemParam>::State {
        // SAFETY: requirements are upheld by caller
        unsafe { self.0.build(world, shared_states) }
    }
}

/// A [`SystemParamBuilder`] for a [`If`].
#[derive(Clone)]
pub struct IfBuilder<T>(T);

// SAFETY: `IfBuilder<B>` builds a state that is valid for `P`, and any state valid for `P` is valid for `If<P>`
unsafe impl<P: SystemParam, B: SystemParamBuilder<P>> SystemParamBuilder<If<P>> for IfBuilder<B> {
    unsafe fn build(
        self,
        world: &mut World,
        shared_states: &SharedStates,
    ) -> <If<P> as SystemParam>::State {
        // SAFETY: requirements are upheld by caller
        unsafe { self.0.build(world, shared_states) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entities,
        error::Result,
        prelude::{Component, Query},
        reflect::ReflectResource,
        system::{Local, RunSystemOnce},
    };
    use alloc::vec;
    use bevy_reflect::Reflect;

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

    fn query_system_result(query: Query<()>) -> Result<usize> {
        Ok(query.iter().count())
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

        let builder_system = (LocalBuilder(10),).build_system(local_system);

        let output = world.run_system_once(builder_system).unwrap();
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

        let builder_system = (QueryParamBuilder::new(|query| {
            query.with::<A>();
        }),)
            .build_system(query_system);

        let output = world.run_system_once(builder_system).unwrap();
        assert_eq!(output, 1);
    }

    #[test]
    fn query_builder_system_result_fallible() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (QueryParamBuilder::new(|query| {
            query.with::<A>();
        }),)
            .build_state(&mut world)
            .build_system(query_system_result);

        // The type annotation here is necessary since the system
        // could also return `Result<usize>`
        let output: usize = world.run_system_once(system).unwrap();
        assert_eq!(output, 1);

        let builder_system = (QueryParamBuilder::new(|query| {
            query.with::<A>();
        }),)
            .build_system(query_system_result);

        // The type annotation here is necessary since the system
        // could also return `Result<usize>`
        let output: usize = world.run_system_once(builder_system).unwrap();
        assert_eq!(output, 1);
    }

    #[test]
    fn query_builder_result_infallible() {
        let mut world = World::new();

        world.spawn(A);
        world.spawn_empty();

        let system = (QueryParamBuilder::new(|query| {
            query.with::<A>();
        }),)
            .build_state(&mut world)
            .build_system(query_system_result);

        // The type annotation here is necessary since the system
        // could also return `usize`
        let output: Result<usize> = world.run_system_once(system).unwrap();
        assert_eq!(output.unwrap(), 1);

        let builder_system = (QueryParamBuilder::new(|query| {
            query.with::<A>();
        }),)
            .build_system(query_system_result);

        // The type annotation here is necessary since the system
        // could also return `usize`
        let output: Result<usize> = world.run_system_once(builder_system).unwrap();
        assert_eq!(output.unwrap(), 1);
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

        let state = QueryBuilder::new(&mut world).with::<A>().build();

        let builder_system = (state,).build_system(query_system);

        let output = world.run_system_once(builder_system).unwrap();
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

        let builder_system = (LocalBuilder(0), ParamBuilder).build_system(multi_param_system);

        let output = world.run_system_once(builder_system).unwrap();
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

        // NOTE: this isn't compatible with `BuilderSystem`, because the system param builder isn't 'static

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

        // NOTE: this isn't compatible with `BuilderSystem`, because it uses parameter type inference

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

        let builder_system = (ParamSetBuilder((
            QueryParamBuilder::new(|builder| {
                builder.with::<B>();
            }),
            QueryParamBuilder::new(|builder| {
                builder.with::<C>();
            }),
        )),)
            .build_system(|mut params: ParamSet<(Query<&mut A>, Query<&mut A>)>| {
                params.p0().iter().count() + params.p1().iter().count()
            });

        let output = world.run_system_once(builder_system).unwrap();
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

        // NOTE: this isn't compatible with `BuilderSystem`, because the system param builder isn't 'static

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

        // NOTE: this isn't compatible with `BuilderSystem`, because the system param builder isn't 'static

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

        let builder_system = (CustomParamBuilder {
            local: LocalBuilder(100),
            query: QueryParamBuilder::new(|builder| {
                builder.with::<A>();
            }),
        },)
            .build_system(|param: CustomParam| *param.local + param.query.iter().count());

        let output = world.run_system_once(builder_system).unwrap();
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
}
