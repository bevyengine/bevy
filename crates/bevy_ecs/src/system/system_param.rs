pub use crate::change_detection::{NonSendMut, Res, ResMut};
use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundles,
    change_detection::{Ticks, TicksMut},
    component::{ComponentId, ComponentTicks, Components, Tick},
    entity::Entities,
    query::{
        Access, AccessConflicts, FilteredAccess, FilteredAccessSet, QueryData, QueryFilter,
        QuerySingleError, QueryState, ReadOnlyQueryData,
    },
    storage::{ResourceData, SparseSetIndex},
    system::{Query, Single, SystemMeta},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FromWorld, World},
};
use bevy_ecs_macros::impl_param_set;
pub use bevy_ecs_macros::{Resource, SystemParam};
use bevy_ptr::UnsafeCellDeref;
use bevy_utils::{all_tuples, synccell::SyncCell};
#[cfg(feature = "track_change_detection")]
use core::panic::Location;
use core::{
    any::Any,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use super::Populated;

/// A parameter that can be used in a [`System`](super::System).
///
/// # Derive
///
/// This trait can be derived with the [`derive@super::SystemParam`] macro.
/// This macro only works if each field on the derived struct implements [`SystemParam`].
/// Note: There are additional requirements on the field types.
/// See the *Generic `SystemParam`s* section for details and workarounds of the probable
/// cause if this derive causes an error to be emitted.
///
/// Derived `SystemParam` structs may have two lifetimes: `'w` for data stored in the [`World`],
/// and `'s` for data stored in the parameter's state.
///
/// The following list shows the most common [`SystemParam`]s and which lifetime they require
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// # #[derive(Event)]
/// # struct SomeEvent;
/// # #[derive(Resource)]
/// # struct SomeOtherResource;
/// # use bevy_ecs::system::SystemParam;
/// # #[derive(SystemParam)]
/// # struct ParamsExample<'w, 's> {
/// #    query:
/// Query<'w, 's, Entity>,
/// #    res:
/// Res<'w, SomeResource>,
/// #    res_mut:
/// ResMut<'w, SomeOtherResource>,
/// #    local:
/// Local<'s, u8>,
/// #    commands:
/// Commands<'w, 's>,
/// #    eventreader:
/// EventReader<'w, 's, SomeEvent>,
/// #    eventwriter:
/// EventWriter<'w, SomeEvent>
/// # }
/// ```
/// ## `PhantomData`
///
/// [`PhantomData`] is a special type of `SystemParam` that does nothing.
/// This is useful for constraining generic types or lifetimes.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// use std::marker::PhantomData;
/// use bevy_ecs::system::SystemParam;
///
/// #[derive(SystemParam)]
/// struct MyParam<'w, Marker: 'static> {
///     foo: Res<'w, SomeResource>,
///     marker: PhantomData<Marker>,
/// }
///
/// fn my_system<T: 'static>(param: MyParam<T>) {
///     // Access the resource through `param.foo`
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system::<()>);
/// ```
///
/// # Generic `SystemParam`s
///
/// When using the derive macro, you may see an error in the form of:
///
/// ```text
/// expected ... [ParamType]
/// found associated type `<[ParamType] as SystemParam>::Item<'_, '_>`
/// ```
/// where `[ParamType]` is the type of one of your fields.
/// To solve this error, you can wrap the field of type `[ParamType]` with [`StaticSystemParam`]
/// (i.e. `StaticSystemParam<[ParamType]>`).
///
/// ## Details
///
/// The derive macro requires that the [`SystemParam`] implementation of
/// each field `F`'s [`Item`](`SystemParam::Item`)'s is itself `F`
/// (ignoring lifetimes for simplicity).
/// This assumption is due to type inference reasons, so that the derived [`SystemParam`] can be
/// used as an argument to a function system.
/// If the compiler cannot validate this property for `[ParamType]`, it will error in the form shown above.
///
/// This will most commonly occur when working with `SystemParam`s generically, as the requirement
/// has not been proven to the compiler.
///
/// ## Builders
///
/// If you want to use a [`SystemParamBuilder`](crate::system::SystemParamBuilder) with a derived [`SystemParam`] implementation,
/// add a `#[system_param(builder)]` attribute to the struct.
/// This will generate a builder struct whose name is the param struct suffixed with `Builder`.
/// The builder will not be `pub`, so you may want to expose a method that returns an `impl SystemParamBuilder<T>`.
///
/// ```
/// mod custom_param {
/// #     use bevy_ecs::{
/// #         prelude::*,
/// #         system::{LocalBuilder, QueryParamBuilder, SystemParam},
/// #     };
/// #
///     #[derive(SystemParam)]
///     #[system_param(builder)]
///     pub struct CustomParam<'w, 's> {
///         query: Query<'w, 's, ()>,
///         local: Local<'s, usize>,
///     }
///
///     impl<'w, 's> CustomParam<'w, 's> {
///         pub fn builder(
///             local: usize,
///             query: impl FnOnce(&mut QueryBuilder<()>),
///         ) -> impl SystemParamBuilder<Self> {
///             CustomParamBuilder {
///                 local: LocalBuilder(local),
///                 query: QueryParamBuilder::new(query),
///             }
///         }
///     }
/// }
///
/// use custom_param::CustomParam;
///
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// #
/// let system = (CustomParam::builder(100, |builder| {
///     builder.with::<A>();
/// }),)
///     .build_state(&mut world)
///     .build_system(|param: CustomParam| {});
/// ```
///
/// # Safety
///
/// The implementor must ensure the following is true.
/// - [`SystemParam::init_state`] correctly registers all [`World`] accesses used
///   by [`SystemParam::get_param`] with the provided [`system_meta`](SystemMeta).
/// - None of the world accesses may conflict with any prior accesses registered
///   on `system_meta`.
pub unsafe trait SystemParam: Sized {
    /// Used to store data which persists across invocations of a system.
    type State: Send + Sync + 'static;

    /// The item type returned when constructing this system param.
    /// The value of this associated type should be `Self`, instantiated with new lifetimes.
    ///
    /// You could think of [`SystemParam::Item<'w, 's>`] as being an *operation* that changes the lifetimes bound to `Self`.
    type Item<'world, 'state>: SystemParam<State = Self::State>;

    /// Registers any [`World`] access used by this [`SystemParam`]
    /// and creates a new instance of this param's [`State`](SystemParam::State).
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State;

    /// For the specified [`Archetype`], registers the components accessed by this [`SystemParam`] (if applicable).a
    ///
    /// # Safety
    /// `archetype` must be from the [`World`] used to initialize `state` in [`SystemParam::init_state`].
    #[inline]
    #[allow(unused_variables)]
    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
    }

    /// Applies any deferred mutations stored in this [`SystemParam`]'s state.
    /// This is used to apply [`Commands`] during [`apply_deferred`](crate::prelude::apply_deferred).
    ///
    /// [`Commands`]: crate::prelude::Commands
    #[inline]
    #[allow(unused_variables)]
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {}

    /// Queues any deferred mutations to be applied at the next [`apply_deferred`](crate::prelude::apply_deferred).
    #[inline]
    #[allow(unused_variables)]
    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {}

    /// Validates that the param can be acquired by the [`get_param`](SystemParam::get_param).
    /// Built-in executors use this to prevent systems with invalid params from running.
    /// For nested [`SystemParam`]s validation will fail if any
    /// delegated validation fails.
    ///
    /// However calling and respecting [`SystemParam::validate_param`]
    /// is not a strict requirement, [`SystemParam::get_param`] should
    /// provide it's own safety mechanism to prevent undefined behavior.
    ///
    /// The [`world`](UnsafeWorldCell) can only be used to read param's data
    /// and world metadata. No data can be written.
    ///
    /// When using system parameters that require `change_tick` you can use
    /// [`UnsafeWorldCell::change_tick()`]. Even if this isn't the exact
    /// same tick used for [`SystemParam::get_param`], the world access
    /// ensures that the queried data will be the same in both calls.
    ///
    /// This method has to be called directly before [`SystemParam::get_param`] with no other (relevant)
    /// world mutations inbetween. Otherwise, while it won't lead to any undefined behavior,
    /// the validity of the param may change.
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have read-only access to world data
    ///   registered in [`init_state`](SystemParam::init_state).
    /// - `world` must be the same [`World`] that was used to initialize [`state`](SystemParam::init_state).
    /// - All `world`'s archetypes have been processed by [`new_archetype`](SystemParam::new_archetype).
    unsafe fn validate_param(
        _state: &Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell,
    ) -> bool {
        // By default we allow panics in [`SystemParam::get_param`] and return `true`.
        // Preventing panics is an optional feature.
        true
    }

    /// Creates a parameter to be passed into a [`SystemParamFunction`](super::SystemParamFunction).
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have access to any world data
    ///   registered in [`init_state`](SystemParam::init_state).
    /// - `world` must be the same [`World`] that was used to initialize [`state`](SystemParam::init_state).
    /// - all `world`'s archetypes have been processed by [`new_archetype`](SystemParam::new_archetype).
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state>;
}

/// A [`SystemParam`] that only reads a given [`World`].
///
/// # Safety
/// This must only be implemented for [`SystemParam`] impls that exclusively read the World passed in to [`SystemParam::get_param`]
pub unsafe trait ReadOnlySystemParam: SystemParam {}

/// Shorthand way of accessing the associated type [`SystemParam::Item`] for a given [`SystemParam`].
pub type SystemParamItem<'w, 's, P> = <P as SystemParam>::Item<'w, 's>;

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'w, 's, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReadOnlySystemParam
    for Query<'w, 's, D, F>
{
}

// SAFETY: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Query<'_, '_, D, F> {
    type State = QueryState<D, F>;
    type Item<'w, 's> = Query<'w, 's, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let state = QueryState::new_with_access(world, &mut system_meta.archetype_component_access);
        init_query_param(world, system_meta, &state);
        state
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        state.new_archetype(archetype, &mut system_meta.archetype_component_access);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        unsafe { Query::new(world, state, system_meta.last_run, change_tick) }
    }
}

pub(crate) fn init_query_param<D: QueryData + 'static, F: QueryFilter + 'static>(
    world: &mut World,
    system_meta: &mut SystemMeta,
    state: &QueryState<D, F>,
) {
    assert_component_access_compatibility(
        &system_meta.name,
        core::any::type_name::<D>(),
        core::any::type_name::<F>(),
        &system_meta.component_access_set,
        &state.component_access,
        world,
    );
    system_meta
        .component_access_set
        .add(state.component_access.clone());
}

fn assert_component_access_compatibility(
    system_name: &str,
    query_type: &'static str,
    filter_type: &'static str,
    system_access: &FilteredAccessSet<ComponentId>,
    current: &FilteredAccess<ComponentId>,
    world: &World,
) {
    let conflicts = system_access.get_conflicts_single(current);
    if conflicts.is_empty() {
        return;
    }
    let accesses = match conflicts {
        AccessConflicts::All => "",
        AccessConflicts::Individual(indices) => &format!(
            " {}",
            indices
                .ones()
                .map(|index| world
                    .components
                    .get_info(ComponentId::get_sparse_set_index(index))
                    .unwrap()
                    .name())
                .collect::<Vec<&str>>()
                .join(", ")
        ),
    };
    panic!("error[B0001]: Query<{query_type}, {filter_type}> in system {system_name} accesses component(s){accesses} in a way that conflicts with a previous system parameter. Consider using `Without<T>` to create disjoint Queries or merging conflicting Queries into a `ParamSet`. See: https://bevyengine.org/learn/errors/b0001");
}

// SAFETY: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<'a, D: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Single<'a, D, F> {
    type State = QueryState<D, F>;
    type Item<'w, 's> = Single<'w, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Query::init_state(world, system_meta)
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        // SAFETY: Delegate to existing `SystemParam` implementations.
        unsafe { Query::new_archetype(state, archetype, system_meta) };
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        state.validate_world(world.id());
        // SAFETY: State ensures that the components it accesses are not accessible somewhere elsewhere.
        let result =
            unsafe { state.get_single_unchecked_manual(world, system_meta.last_run, change_tick) };
        let single =
            result.expect("The query was expected to contain exactly one matching entity.");
        Single {
            item: single,
            _filter: PhantomData,
        }
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        state.validate_world(world.id());
        // SAFETY: State ensures that the components it accesses are not mutably accessible elsewhere
        // and the query is read only.
        let result = unsafe {
            state.as_readonly().get_single_unchecked_manual(
                world,
                system_meta.last_run,
                world.change_tick(),
            )
        };
        result.is_ok()
    }
}

// SAFETY: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<'a, D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for Option<Single<'a, D, F>>
{
    type State = QueryState<D, F>;
    type Item<'w, 's> = Option<Single<'w, D, F>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Single::init_state(world, system_meta)
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        // SAFETY: Delegate to existing `SystemParam` implementations.
        unsafe { Single::new_archetype(state, archetype, system_meta) };
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        state.validate_world(world.id());
        // SAFETY: State ensures that the components it accesses are not accessible elsewhere.
        let result =
            unsafe { state.get_single_unchecked_manual(world, system_meta.last_run, change_tick) };
        match result {
            Ok(single) => Some(Single {
                item: single,
                _filter: PhantomData,
            }),
            Err(QuerySingleError::NoEntities(_)) => None,
            Err(QuerySingleError::MultipleEntities(e)) => panic!("{}", e),
        }
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        state.validate_world(world.id());
        // SAFETY: State ensures that the components it accesses are not mutably accessible elsewhere
        // and the query is read only.
        let result = unsafe {
            state.as_readonly().get_single_unchecked_manual(
                world,
                system_meta.last_run,
                world.change_tick(),
            )
        };
        !matches!(result, Err(QuerySingleError::MultipleEntities(_)))
    }
}

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'a, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReadOnlySystemParam
    for Single<'a, D, F>
{
}

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'a, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReadOnlySystemParam
    for Option<Single<'a, D, F>>
{
}

// SAFETY: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for Populated<'_, '_, D, F>
{
    type State = QueryState<D, F>;
    type Item<'w, 's> = Populated<'w, 's, D, F>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Query::init_state(world, system_meta)
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        // SAFETY: Delegate to existing `SystemParam` implementations.
        unsafe { Query::new_archetype(state, archetype, system_meta) };
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: Delegate to existing `SystemParam` implementations.
        let query = unsafe { Query::get_param(state, system_meta, world, change_tick) };
        Populated(query)
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        state.validate_world(world.id());
        // SAFETY:
        // - We have read-only access to the components accessed by query.
        // - The world has been validated.
        !unsafe {
            state.is_empty_unsafe_world_cell(world, system_meta.last_run, world.change_tick())
        }
    }
}

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'w, 's, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReadOnlySystemParam
    for Populated<'w, 's, D, F>
{
}

/// A collection of potentially conflicting [`SystemParam`]s allowed by disjoint access.
///
/// Allows systems to safely access and interact with up to 8 mutually exclusive [`SystemParam`]s, such as
/// two queries that reference the same mutable data or an event reader and writer of the same type.
///
/// Each individual [`SystemParam`] can be accessed by using the functions `p0()`, `p1()`, ..., `p7()`,
/// according to the order they are defined in the `ParamSet`. This ensures that there's either
/// only one mutable reference to a parameter at a time or any number of immutable references.
///
/// # Examples
///
/// The following system mutably accesses the same component two times,
/// which is not allowed due to rust's mutability rules.
///
/// ```should_panic
/// # use bevy_ecs::prelude::*;
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
/// // This will panic at runtime when the system gets initialized.
/// fn bad_system(
///     mut enemies: Query<&mut Health, With<Enemy>>,
///     mut allies: Query<&mut Health, With<Ally>>,
/// ) {
///     // ...
/// }
/// #
/// # let mut bad_system_system = IntoSystem::into_system(bad_system);
/// # let mut world = World::new();
/// # bad_system_system.initialize(&mut world);
/// # bad_system_system.run((), &mut world);
/// ```
///
/// Conflicting `SystemParam`s like these can be placed in a `ParamSet`,
/// which leverages the borrow checker to ensure that only one of the contained parameters are accessed at a given time.
///
/// ```
/// # use bevy_ecs::prelude::*;
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
/// // Given the following system
/// fn fancy_system(
///     mut set: ParamSet<(
///         Query<&mut Health, With<Enemy>>,
///         Query<&mut Health, With<Ally>>,
///     )>
/// ) {
///     // This will access the first `SystemParam`.
///     for mut health in set.p0().iter_mut() {
///         // Do your fancy stuff here...
///     }
///
///     // The second `SystemParam`.
///     // This would fail to compile if the previous parameter was still borrowed.
///     for mut health in set.p1().iter_mut() {
///         // Do even fancier stuff here...
///     }
/// }
/// # bevy_ecs::system::assert_is_system(fancy_system);
/// ```
///
/// Of course, `ParamSet`s can be used with any kind of `SystemParam`, not just [queries](Query).
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Event)]
/// # struct MyEvent;
/// # impl MyEvent {
/// #   pub fn new() -> Self { Self }
/// # }
/// fn event_system(
///     mut set: ParamSet<(
///         // `EventReader`s and `EventWriter`s conflict with each other,
///         // since they both access the event queue resource for `MyEvent`.
///         EventReader<MyEvent>,
///         EventWriter<MyEvent>,
///         // `&World` reads the entire world, so a `ParamSet` is the only way
///         // that it can be used in the same system as any mutable accesses.
///         &World,
///     )>,
/// ) {
///     for event in set.p0().read() {
///         // ...
///         # let _event = event;
///     }
///     set.p1().send(MyEvent::new());
///
///     let entities = set.p2().entities();
///     // ...
///     # let _entities = entities;
/// }
/// # bevy_ecs::system::assert_is_system(event_system);
/// ```
pub struct ParamSet<'w, 's, T: SystemParam> {
    param_states: &'s mut T::State,
    world: UnsafeWorldCell<'w>,
    system_meta: SystemMeta,
    change_tick: Tick,
}

impl_param_set!();

/// A type that can be inserted into a [`World`] as a singleton.
///
/// You can access resource data in systems using the [`Res`] and [`ResMut`] system parameters
///
/// Only one resource of each type can be stored in a [`World`] at any given time.
///
/// # Examples
///
/// ```
/// # let mut world = World::default();
/// # let mut schedule = Schedule::default();
/// # use bevy_ecs::prelude::*;
/// #[derive(Resource)]
/// struct MyResource { value: u32 }
///
/// world.insert_resource(MyResource { value: 42 });
///
/// fn read_resource_system(resource: Res<MyResource>) {
///     assert_eq!(resource.value, 42);
/// }
///
/// fn write_resource_system(mut resource: ResMut<MyResource>) {
///     assert_eq!(resource.value, 42);
///     resource.value = 0;
///     assert_eq!(resource.value, 0);
/// }
/// # schedule.add_systems((read_resource_system, write_resource_system).chain());
/// # schedule.run(&mut world);
/// ```
///
/// # `!Sync` Resources
/// A `!Sync` type cannot implement `Resource`. However, it is possible to wrap a `Send` but not `Sync`
/// type in [`SyncCell`] or the currently unstable [`Exclusive`] to make it `Sync`. This forces only
/// having mutable access (`&mut T` only, never `&T`), but makes it safe to reference across multiple
/// threads.
///
/// This will fail to compile since `RefCell` is `!Sync`.
/// ```compile_fail
/// # use std::cell::RefCell;
/// # use bevy_ecs::system::Resource;
///
/// #[derive(Resource)]
/// struct NotSync {
///    counter: RefCell<usize>,
/// }
/// ```
///
/// This will compile since the `RefCell` is wrapped with `SyncCell`.
/// ```
/// # use std::cell::RefCell;
/// # use bevy_ecs::system::Resource;
/// use bevy_utils::synccell::SyncCell;
///
/// #[derive(Resource)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Resource`",
    label = "invalid `Resource`",
    note = "consider annotating `{Self}` with `#[derive(Resource)]`"
)]
pub trait Resource: Send + Sync + 'static {}

// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Res<'a, T> {}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = Res<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let component_id = world.components.register_resource::<T>();
        let archetype_component_id = world.initialize_resource_internal(component_id).id();

        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_resource_write(component_id),
            "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002",
            core::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_resource_read(component_id);

        system_meta
            .archetype_component_access
            .add_resource_read(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn validate_param(
        &component_id: &Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        // SAFETY: Read-only access to resource metadata.
        unsafe { world.storages() }
            .resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks, _caller) =
            world
                .get_resource_with_ticks(component_id)
                .unwrap_or_else(|| {
                    panic!(
                        "Resource requested by {} does not exist: {}",
                        system_meta.name,
                        core::any::type_name::<T>()
                    )
                });
        Res {
            value: ptr.deref(),
            ticks: Ticks {
                added: ticks.added.deref(),
                changed: ticks.changed.deref(),
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
            #[cfg(feature = "track_change_detection")]
            changed_by: _caller.deref(),
        }
    }
}

// SAFETY: Only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Option<Res<'a, T>> {}

// SAFETY: this impl defers to `Res`, which initializes and validates the correct world access.
unsafe impl<'a, T: Resource> SystemParam for Option<Res<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<Res<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        Res::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_with_ticks(component_id)
            .map(|(ptr, ticks, _caller)| Res {
                value: ptr.deref(),
                ticks: Ticks {
                    added: ticks.added.deref(),
                    changed: ticks.changed.deref(),
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
                #[cfg(feature = "track_change_detection")]
                changed_by: _caller.deref(),
            })
    }
}

// SAFETY: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = ResMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let component_id = world.components.register_resource::<T>();
        let archetype_component_id = world.initialize_resource_internal(component_id).id();

        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_resource_write(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002",
                core::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_resource_read(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002",
                core::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_resource_write(component_id);

        system_meta
            .archetype_component_access
            .add_resource_write(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn validate_param(
        &component_id: &Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        // SAFETY: Read-only access to resource metadata.
        unsafe { world.storages() }
            .resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let value = world
            .get_resource_mut_by_id(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    core::any::type_name::<T>()
                )
            });
        ResMut {
            value: value.value.deref_mut::<T>(),
            ticks: TicksMut {
                added: value.ticks.added,
                changed: value.ticks.changed,
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
            #[cfg(feature = "track_change_detection")]
            changed_by: value.changed_by,
        }
    }
}

// SAFETY: this impl defers to `ResMut`, which initializes and validates the correct world access.
unsafe impl<'a, T: Resource> SystemParam for Option<ResMut<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<ResMut<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        ResMut::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_resource_mut_by_id(component_id)
            .map(|value| ResMut {
                value: value.value.deref_mut::<T>(),
                ticks: TicksMut {
                    added: value.ticks.added,
                    changed: value.ticks.changed,
                    last_run: system_meta.last_run,
                    this_run: change_tick,
                },
                #[cfg(feature = "track_change_detection")]
                changed_by: value.changed_by,
            })
    }
}

/// SAFETY: only reads world
unsafe impl<'w> ReadOnlySystemParam for &'w World {}

// SAFETY: `read_all` access is set and conflicts result in a panic
unsafe impl SystemParam for &'_ World {
    type State = ();
    type Item<'w, 's> = &'w World;

    fn init_state(_world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let mut access = Access::default();
        access.read_all();
        if !system_meta
            .archetype_component_access
            .is_compatible(&access)
        {
            panic!("&World conflicts with a previous mutable system parameter. Allowing this would break Rust's mutability rules");
        }
        system_meta.archetype_component_access.extend(&access);

        let mut filtered_access = FilteredAccess::default();

        filtered_access.read_all();
        if !system_meta
            .component_access_set
            .get_conflicts_single(&filtered_access)
            .is_empty()
        {
            panic!("&World conflicts with a previous mutable system parameter. Allowing this would break Rust's mutability rules");
        }
        system_meta.component_access_set.add(filtered_access);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: Read-only access to the entire world was registered in `init_state`.
        unsafe { world.world() }
    }
}

/// SAFETY: `DeferredWorld` can read all components and resources but cannot be used to gain any other mutable references.
unsafe impl<'w> SystemParam for DeferredWorld<'w> {
    type State = ();
    type Item<'world, 'state> = DeferredWorld<'world>;

    fn init_state(_world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.component_access_set.read_all();
        system_meta.component_access_set.write_all();
        system_meta.set_has_deferred();
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        world.into_deferred()
    }
}

/// A system local [`SystemParam`].
///
/// A local may only be accessed by the system itself and is therefore not visible to other systems.
/// If two or more systems specify the same local type each will have their own unique local.
/// If multiple [`SystemParam`]s within the same system each specify the same local type
/// each will get their own distinct data storage.
///
/// The supplied lifetime parameter is the [`SystemParam`]s `'s` lifetime.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let world = &mut World::default();
/// fn write_to_local(mut local: Local<usize>) {
///     *local = 42;
/// }
/// fn read_from_local(local: Local<usize>) -> usize {
///     *local
/// }
/// let mut write_system = IntoSystem::into_system(write_to_local);
/// let mut read_system = IntoSystem::into_system(read_from_local);
/// write_system.initialize(world);
/// read_system.initialize(world);
///
/// assert_eq!(read_system.run((), world), 0);
/// write_system.run((), world);
/// // Note how the read local is still 0 due to the locals not being shared.
/// assert_eq!(read_system.run((), world), 0);
/// ```
///
/// N.B. A [`Local`]s value cannot be read or written to outside of the containing system.
/// To add configuration to a system, convert a capturing closure into the system instead:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::assert_is_system;
/// struct Config(u32);
/// #[derive(Resource)]
/// struct MyU32Wrapper(u32);
/// fn reset_to_system(value: Config) -> impl FnMut(ResMut<MyU32Wrapper>) {
///     move |mut val| val.0 = value.0
/// }
///
/// // .add_systems(reset_to_system(my_config))
/// # assert_is_system(reset_to_system(Config(10)));
/// ```
#[derive(Debug)]
pub struct Local<'s, T: FromWorld + Send + 'static>(pub(crate) &'s mut T);

// SAFETY: Local only accesses internal state
unsafe impl<'s, T: FromWorld + Send + 'static> ReadOnlySystemParam for Local<'s, T> {}

impl<'s, T: FromWorld + Send + 'static> Deref for Local<'s, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'s, T: FromWorld + Send + 'static> DerefMut for Local<'s, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'s, 'a, T: FromWorld + Send + 'static> IntoIterator for &'a Local<'s, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'s, 'a, T: FromWorld + Send + 'static> IntoIterator for &'a mut Local<'s, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// SAFETY: only local state is accessed
unsafe impl<'a, T: FromWorld + Send + 'static> SystemParam for Local<'a, T> {
    type State = SyncCell<T>;
    type Item<'w, 's> = Local<'s, T>;

    fn init_state(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(T::from_world(world))
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        Local(state.get())
    }
}

/// Types that can be used with [`Deferred<T>`] in systems.
/// This allows storing system-local data which is used to defer [`World`] mutations.
///
/// Types that implement `SystemBuffer` should take care to perform as many
/// computations up-front as possible. Buffers cannot be applied in parallel,
/// so you should try to minimize the time spent in [`SystemBuffer::apply`].
pub trait SystemBuffer: FromWorld + Send + 'static {
    /// Applies any deferred mutations to the [`World`].
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World);
    /// Queues any deferred mutations to be applied at the next [`apply_deferred`](crate::prelude::apply_deferred).
    fn queue(&mut self, _system_meta: &SystemMeta, _world: DeferredWorld) {}
}

/// A [`SystemParam`] that stores a buffer which gets applied to the [`World`] during
/// [`apply_deferred`](crate::schedule::apply_deferred).
/// This is used internally by [`Commands`] to defer `World` mutations.
///
/// [`Commands`]: crate::system::Commands
///
/// # Examples
///
/// By using this type to defer mutations, you can avoid mutable `World` access within
/// a system, which allows it to run in parallel with more systems.
///
/// Note that deferring mutations is *not* free, and should only be used if
/// the gains in parallelization outweigh the time it takes to apply deferred mutations.
/// In general, [`Deferred`] should only be used for mutations that are infrequent,
/// or which otherwise take up a small portion of a system's run-time.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// // Tracks whether or not there is a threat the player should be aware of.
/// #[derive(Resource, Default)]
/// pub struct Alarm(bool);
///
/// #[derive(Component)]
/// pub struct Settlement {
///     // ...
/// }
///
/// // A threat from inside the settlement.
/// #[derive(Component)]
/// pub struct Criminal;
///
/// // A threat from outside the settlement.
/// #[derive(Component)]
/// pub struct Monster;
///
/// # impl Criminal { pub fn is_threat(&self, _: &Settlement) -> bool { true } }
///
/// use bevy_ecs::system::{Deferred, SystemBuffer, SystemMeta};
///
/// // Uses deferred mutations to allow signalling the alarm from multiple systems in parallel.
/// #[derive(Resource, Default)]
/// struct AlarmFlag(bool);
///
/// impl AlarmFlag {
///     /// Sounds the alarm the next time buffers are applied via apply_deferred.
///     pub fn flag(&mut self) {
///         self.0 = true;
///     }
/// }
///
/// impl SystemBuffer for AlarmFlag {
///     // When `AlarmFlag` is used in a system, this function will get
///     // called the next time buffers are applied via apply_deferred.
///     fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
///         if self.0 {
///             world.resource_mut::<Alarm>().0 = true;
///             self.0 = false;
///         }
///     }
/// }
///
/// // Sound the alarm if there are any criminals who pose a threat.
/// fn alert_criminal(
///     settlements: Query<&Settlement>,
///     criminals: Query<&Criminal>,
///     mut alarm: Deferred<AlarmFlag>
/// ) {
///     let settlement = settlements.single();
///     for criminal in &criminals {
///         // Only sound the alarm if the criminal is a threat.
///         // For this example, assume that this check is expensive to run.
///         // Since the majority of this system's run-time is dominated
///         // by calling `is_threat()`, we defer sounding the alarm to
///         // allow this system to run in parallel with other alarm systems.
///         if criminal.is_threat(settlement) {
///             alarm.flag();
///         }
///     }
/// }
///
/// // Sound the alarm if there is a monster.
/// fn alert_monster(
///     monsters: Query<&Monster>,
///     mut alarm: ResMut<Alarm>
/// ) {
///     if monsters.iter().next().is_some() {
///         // Since this system does nothing except for sounding the alarm,
///         // it would be pointless to defer it, so we sound the alarm directly.
///         alarm.0 = true;
///     }
/// }
///
/// let mut world = World::new();
/// world.init_resource::<Alarm>();
/// world.spawn(Settlement {
///     // ...
/// });
///
/// let mut schedule = Schedule::default();
/// // These two systems have no conflicts and will run in parallel.
/// schedule.add_systems((alert_criminal, alert_monster));
///
/// // There are no criminals or monsters, so the alarm is not sounded.
/// schedule.run(&mut world);
/// assert_eq!(world.resource::<Alarm>().0, false);
///
/// // Spawn a monster, which will cause the alarm to be sounded.
/// let m_id = world.spawn(Monster).id();
/// schedule.run(&mut world);
/// assert_eq!(world.resource::<Alarm>().0, true);
///
/// // Remove the monster and reset the alarm.
/// world.entity_mut(m_id).despawn();
/// world.resource_mut::<Alarm>().0 = false;
///
/// // Spawn a criminal, which will cause the alarm to be sounded.
/// world.spawn(Criminal);
/// schedule.run(&mut world);
/// assert_eq!(world.resource::<Alarm>().0, true);
/// ```
pub struct Deferred<'a, T: SystemBuffer>(pub(crate) &'a mut T);

impl<'a, T: SystemBuffer> Deref for Deferred<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T: SystemBuffer> DerefMut for Deferred<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<T: SystemBuffer> Deferred<'_, T> {
    /// Returns a [`Deferred<T>`] with a smaller lifetime.
    /// This is useful if you have `&mut Deferred<T>` but need `Deferred<T>`.
    pub fn reborrow(&mut self) -> Deferred<T> {
        Deferred(self.0)
    }
}

// SAFETY: Only local state is accessed.
unsafe impl<T: SystemBuffer> ReadOnlySystemParam for Deferred<'_, T> {}

// SAFETY: Only local state is accessed.
unsafe impl<T: SystemBuffer> SystemParam for Deferred<'_, T> {
    type State = SyncCell<T>;
    type Item<'w, 's> = Deferred<'s, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_has_deferred();
        SyncCell::new(T::from_world(world))
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        state.get().apply(system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        state.get().queue(system_meta, world);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        Deferred(state.get())
    }
}

/// Shared borrow of a non-[`Send`] resource.
///
/// Only `Send` resources may be accessed with the [`Res`] [`SystemParam`]. In case that the
/// resource does not implement `Send`, this `SystemParam` wrapper can be used. This will instruct
/// the scheduler to instead run the system on the main thread so that it doesn't send the resource
/// over to another thread.
///
/// This [`SystemParam`] fails validation if non-send resource doesn't exist.
/// This will cause systems that use this parameter to be skipped.
///
/// Use [`Option<NonSend<T>>`] instead if the resource might not always exist.
pub struct NonSend<'w, T: 'static> {
    pub(crate) value: &'w T,
    ticks: ComponentTicks,
    last_run: Tick,
    this_run: Tick,
    #[cfg(feature = "track_change_detection")]
    changed_by: &'static Location<'static>,
}

// SAFETY: Only reads a single World non-send resource
unsafe impl<'w, T> ReadOnlySystemParam for NonSend<'w, T> {}

impl<'w, T> Debug for NonSend<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("NonSend").field(&self.value).finish()
    }
}

impl<'w, T: 'static> NonSend<'w, T> {
    /// Returns `true` if the resource was added after the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_run, self.this_run)
    }

    /// Returns `true` if the resource was added or mutably dereferenced after the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_changed(self.last_run, self.this_run)
    }

    /// The location that last caused this to change.
    #[cfg(feature = "track_change_detection")]
    pub fn changed_by(&self) -> &'static Location<'static> {
        self.changed_by
    }
}

impl<'w, T> Deref for NonSend<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}
impl<'a, T> From<NonSendMut<'a, T>> for NonSend<'a, T> {
    fn from(nsm: NonSendMut<'a, T>) -> Self {
        Self {
            value: nsm.value,
            ticks: ComponentTicks {
                added: nsm.ticks.added.to_owned(),
                changed: nsm.ticks.changed.to_owned(),
            },
            this_run: nsm.ticks.this_run,
            last_run: nsm.ticks.last_run,
            #[cfg(feature = "track_change_detection")]
            changed_by: nsm.changed_by,
        }
    }
}

// SAFETY: NonSendComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSend<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_non_send();

        let component_id = world.components.register_non_send::<T>();
        let archetype_component_id = world.initialize_non_send_internal(component_id).id();

        let combined_access = system_meta.component_access_set.combined_access();
        assert!(
            !combined_access.has_resource_write(component_id),
            "error[B0002]: NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002",
            core::any::type_name::<T>(),
            system_meta.name,
        );
        system_meta
            .component_access_set
            .add_unfiltered_resource_read(component_id);

        system_meta
            .archetype_component_access
            .add_resource_read(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn validate_param(
        &component_id: &Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        // SAFETY: Read-only access to resource metadata.
        unsafe { world.storages() }
            .non_send_resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks, _caller) =
            world
                .get_non_send_with_ticks(component_id)
                .unwrap_or_else(|| {
                    panic!(
                        "Non-send resource requested by {} does not exist: {}",
                        system_meta.name,
                        core::any::type_name::<T>()
                    )
                });

        NonSend {
            value: ptr.deref(),
            ticks: ticks.read(),
            last_run: system_meta.last_run,
            this_run: change_tick,
            #[cfg(feature = "track_change_detection")]
            changed_by: _caller.deref(),
        }
    }
}

// SAFETY: Only reads a single World non-send resource
unsafe impl<T: 'static> ReadOnlySystemParam for Option<NonSend<'_, T>> {}

// SAFETY: this impl defers to `NonSend`, which initializes and validates the correct world access.
unsafe impl<T: 'static> SystemParam for Option<NonSend<'_, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<NonSend<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        NonSend::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_non_send_with_ticks(component_id)
            .map(|(ptr, ticks, _caller)| NonSend {
                value: ptr.deref(),
                ticks: ticks.read(),
                last_run: system_meta.last_run,
                this_run: change_tick,
                #[cfg(feature = "track_change_detection")]
                changed_by: _caller.deref(),
            })
    }
}

// SAFETY: NonSendMut ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSendMut<'w, T>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        system_meta.set_non_send();

        let component_id = world.components.register_non_send::<T>();
        let archetype_component_id = world.initialize_non_send_internal(component_id).id();

        let combined_access = system_meta.component_access_set.combined_access();
        if combined_access.has_component_write(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002",
                core::any::type_name::<T>(), system_meta.name);
        } else if combined_access.has_component_read(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Consider removing the duplicate access. See: https://bevyengine.org/learn/errors/b0002",
                core::any::type_name::<T>(), system_meta.name);
        }
        system_meta
            .component_access_set
            .add_unfiltered_resource_write(component_id);

        system_meta
            .archetype_component_access
            .add_resource_write(archetype_component_id);

        component_id
    }

    #[inline]
    unsafe fn validate_param(
        &component_id: &Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        // SAFETY: Read-only access to resource metadata.
        unsafe { world.storages() }
            .non_send_resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks, _caller) =
            world
                .get_non_send_with_ticks(component_id)
                .unwrap_or_else(|| {
                    panic!(
                        "Non-send resource requested by {} does not exist: {}",
                        system_meta.name,
                        core::any::type_name::<T>()
                    )
                });
        NonSendMut {
            value: ptr.assert_unique().deref_mut(),
            ticks: TicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
            #[cfg(feature = "track_change_detection")]
            changed_by: _caller.deref_mut(),
        }
    }
}

// SAFETY: this impl defers to `NonSendMut`, which initializes and validates the correct world access.
unsafe impl<'a, T: 'static> SystemParam for Option<NonSendMut<'a, T>> {
    type State = ComponentId;
    type Item<'w, 's> = Option<NonSendMut<'w, T>>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        NonSendMut::<T>::init_state(world, system_meta)
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world
            .get_non_send_with_ticks(component_id)
            .map(|(ptr, ticks, _caller)| NonSendMut {
                value: ptr.assert_unique().deref_mut(),
                ticks: TicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
                #[cfg(feature = "track_change_detection")]
                changed_by: _caller.deref_mut(),
            })
    }
}

// SAFETY: Only reads World archetypes
unsafe impl<'a> ReadOnlySystemParam for &'a Archetypes {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Archetypes {
    type State = ();
    type Item<'w, 's> = &'w Archetypes;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.archetypes()
    }
}

// SAFETY: Only reads World components
unsafe impl<'a> ReadOnlySystemParam for &'a Components {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Components {
    type State = ();
    type Item<'w, 's> = &'w Components;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.components()
    }
}

// SAFETY: Only reads World entities
unsafe impl<'a> ReadOnlySystemParam for &'a Entities {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Entities {
    type State = ();
    type Item<'w, 's> = &'w Entities;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.entities()
    }
}

// SAFETY: Only reads World bundles
unsafe impl<'a> ReadOnlySystemParam for &'a Bundles {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Bundles {
    type State = ();
    type Item<'w, 's> = &'w Bundles;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.bundles()
    }
}

/// A [`SystemParam`] that reads the previous and current change ticks of the system.
///
/// A system's change ticks are updated each time it runs:
/// - `last_run` copies the previous value of `change_tick`
/// - `this_run` copies the current value of [`World::read_change_tick`]
///
/// Component change ticks that are more recent than `last_run` will be detected by the system.
/// Those can be read by calling [`last_changed`](crate::change_detection::DetectChanges::last_changed)
/// on a [`Mut<T>`](crate::change_detection::Mut) or [`ResMut<T>`](ResMut).
#[derive(Debug)]
pub struct SystemChangeTick {
    last_run: Tick,
    this_run: Tick,
}

impl SystemChangeTick {
    /// Returns the current [`World`] change tick seen by the system.
    #[inline]
    pub fn this_run(&self) -> Tick {
        self.this_run
    }

    /// Returns the [`World`] change tick seen by the system the previous time it ran.
    #[inline]
    pub fn last_run(&self) -> Tick {
        self.last_run
    }
}

// SAFETY: Only reads internal system state
unsafe impl ReadOnlySystemParam for SystemChangeTick {}

// SAFETY: `SystemChangeTick` doesn't require any world access
unsafe impl SystemParam for SystemChangeTick {
    type State = ();
    type Item<'w, 's> = SystemChangeTick;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'w, 's>(
        _state: &'s mut Self::State,
        system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        SystemChangeTick {
            last_run: system_meta.last_run,
            this_run: change_tick,
        }
    }
}

// SAFETY: When initialized with `init_state`, `get_param` returns an empty `Vec` and does no access.
// Therefore, `init_state` trivially registers all access, and no accesses can conflict.
// Note that the safety requirements for non-empty `Vec`s are handled by the `SystemParamBuilder` impl that builds them.
unsafe impl<T: SystemParam> SystemParam for Vec<T> {
    type State = Vec<T::State>;

    type Item<'world, 'state> = Vec<T::Item<'world, 'state>>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        Vec::new()
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        state
            .iter()
            .all(|state| T::validate_param(state, system_meta, world))
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        state
            .iter_mut()
            // SAFETY:
            // - We initialized the state for each parameter in the builder, so the caller ensures we have access to any world data needed by each param.
            // - The caller ensures this was the world used to initialize our state, and we used that world to initialize parameter states
            .map(|state| unsafe { T::get_param(state, system_meta, world, change_tick) })
            .collect()
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        for state in state {
            // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
            unsafe { T::new_archetype(state, archetype, system_meta) };
        }
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        for state in state {
            T::apply(state, system_meta, world);
        }
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, mut world: DeferredWorld) {
        for state in state {
            T::queue(state, system_meta, world.reborrow());
        }
    }
}

// SAFETY: When initialized with `init_state`, `get_param` returns an empty `Vec` and does no access.
// Therefore, `init_state` trivially registers all access, and no accesses can conflict.
// Note that the safety requirements for non-empty `Vec`s are handled by the `SystemParamBuilder` impl that builds them.
unsafe impl<T: SystemParam> SystemParam for ParamSet<'_, '_, Vec<T>> {
    type State = Vec<T::State>;

    type Item<'world, 'state> = ParamSet<'world, 'state, Vec<T>>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        Vec::new()
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        ParamSet {
            param_states: state,
            system_meta: system_meta.clone(),
            world,
            change_tick,
        }
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        for state in state {
            // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
            unsafe { T::new_archetype(state, archetype, system_meta) }
        }
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        for state in state {
            T::apply(state, system_meta, world);
        }
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, mut world: DeferredWorld) {
        for state in state {
            T::queue(state, system_meta, world.reborrow());
        }
    }
}

impl<T: SystemParam> ParamSet<'_, '_, Vec<T>> {
    /// Accesses the parameter at the given index.
    /// No other parameters may be accessed while this one is active.
    pub fn get_mut(&mut self, index: usize) -> T::Item<'_, '_> {
        // SAFETY:
        // - We initialized the state for each parameter in the builder, so the caller ensures we have access to any world data needed by any param.
        //   We have mutable access to the ParamSet, so no other params in the set are active.
        // - The caller of `get_param` ensured that this was the world used to initialize our state, and we used that world to initialize parameter states
        unsafe {
            T::get_param(
                &mut self.param_states[index],
                &self.system_meta,
                self.world,
                self.change_tick,
            )
        }
    }

    /// Calls a closure for each parameter in the set.
    pub fn for_each(&mut self, mut f: impl FnMut(T::Item<'_, '_>)) {
        self.param_states.iter_mut().for_each(|state| {
            f(
                // SAFETY:
                // - We initialized the state for each parameter in the builder, so the caller ensures we have access to any world data needed by any param.
                //   We have mutable access to the ParamSet, so no other params in the set are active.
                // - The caller of `get_param` ensured that this was the world used to initialize our state, and we used that world to initialize parameter states
                unsafe { T::get_param(state, &self.system_meta, self.world, self.change_tick) },
            );
        });
    }
}

macro_rules! impl_system_param_tuple {
    ($(#[$meta:meta])* $($param: ident),*) => {
        $(#[$meta])*
        // SAFETY: tuple consists only of ReadOnlySystemParams
        unsafe impl<$($param: ReadOnlySystemParam),*> ReadOnlySystemParam for ($($param,)*) {}

        // SAFETY: implementors of each `SystemParam` in the tuple have validated their impls
        #[allow(clippy::undocumented_unsafe_blocks)] // false positive by clippy
        #[allow(non_snake_case)]
        $(#[$meta])*
        unsafe impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'w, 's> = ($($param::Item::<'w, 's>,)*);

            #[inline]
            fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
                (($($param::init_state(_world, _system_meta),)*))
            }

            #[inline]
            #[allow(unused_unsafe)]
            unsafe fn new_archetype(($($param,)*): &mut Self::State, _archetype: &Archetype, _system_meta: &mut SystemMeta) {
                // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
                unsafe { $($param::new_archetype($param, _archetype, _system_meta);)* }
            }

            #[inline]
            fn apply(($($param,)*): &mut Self::State, _system_meta: &SystemMeta, _world: &mut World) {
                $($param::apply($param, _system_meta, _world);)*
            }

            #[inline]
            fn queue(($($param,)*): &mut Self::State, _system_meta: &SystemMeta, mut _world: DeferredWorld) {
                $($param::queue($param, _system_meta, _world.reborrow());)*
            }

            #[inline]
            unsafe fn validate_param(
                state: &Self::State,
                _system_meta: &SystemMeta,
                _world: UnsafeWorldCell,
            ) -> bool {
                let ($($param,)*) = state;
                $($param::validate_param($param, _system_meta, _world)&&)* true
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn get_param<'w, 's>(
                state: &'s mut Self::State,
                _system_meta: &SystemMeta,
                _world: UnsafeWorldCell<'w>,
                _change_tick: Tick,
            ) -> Self::Item<'w, 's> {
                let ($($param,)*) = state;
                ($($param::get_param($param, _system_meta, _world, _change_tick),)*)
            }
        }
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_system_param_tuple,
    0,
    16,
    P
);

/// Contains type aliases for built-in [`SystemParam`]s with `'static` lifetimes.
/// This makes it more convenient to refer to these types in contexts where
/// explicit lifetime annotations are required.
///
/// Note that this is entirely safe and tracks lifetimes correctly.
/// This purely exists for convenience.
///
/// You can't instantiate a static `SystemParam`, you'll always end up with
/// `Res<'w, T>`, `ResMut<'w, T>` or `&'w T` bound to the lifetime of the provided
/// `&'w World`.
///
/// [`SystemParam`]: super::SystemParam
pub mod lifetimeless {
    /// A [`Query`](super::Query) with `'static` lifetimes.
    pub type SQuery<D, F = ()> = super::Query<'static, 'static, D, F>;
    /// A shorthand for writing `&'static T`.
    pub type Read<T> = &'static T;
    /// A shorthand for writing `&'static mut T`.
    pub type Write<T> = &'static mut T;
    /// A [`Res`](super::Res) with `'static` lifetimes.
    pub type SRes<T> = super::Res<'static, T>;
    /// A [`ResMut`](super::ResMut) with `'static` lifetimes.
    pub type SResMut<T> = super::ResMut<'static, T>;
    /// [`Commands`](crate::system::Commands) with `'static` lifetimes.
    pub type SCommands = crate::system::Commands<'static, 'static>;
}

/// A helper for using system parameters in generic contexts
///
/// This type is a [`SystemParam`] adapter which always has
/// `Self::Item == Self` (ignoring lifetimes for brevity),
/// no matter the argument [`SystemParam`] (`P`) (other than
/// that `P` must be `'static`)
///
/// This makes it useful for having arbitrary [`SystemParam`] type arguments
/// to function systems, or for generic types using the [`derive@SystemParam`]
/// derive:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::{SystemParam, StaticSystemParam};
/// #[derive(SystemParam)]
/// struct GenericParam<'w,'s, T: SystemParam + 'static> {
///     field: StaticSystemParam<'w, 's, T>,
/// }
/// fn do_thing_generically<T: SystemParam + 'static>(t: StaticSystemParam<T>) {}
///
/// fn check_always_is_system<T: SystemParam + 'static>(){
///     bevy_ecs::system::assert_is_system(do_thing_generically::<T>);
/// }
/// ```
/// Note that in a real case you'd generally want
/// additional bounds on `P`, for your use of the parameter
/// to have a reason to be generic.
///
/// For example, using this would allow a type to be generic over
/// whether a resource is accessed mutably or not, with
/// impls being bounded on [`P: Deref<Target=MyType>`](Deref), and
/// [`P: DerefMut<Target=MyType>`](DerefMut) depending on whether the
/// method requires mutable access or not.
///
/// The method which doesn't use this type will not compile:
/// ```compile_fail
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::{SystemParam, StaticSystemParam};
///
/// fn do_thing_generically<T: SystemParam + 'static>(t: T) {}
///
/// #[derive(SystemParam)]
/// struct GenericParam<'w, 's, T: SystemParam> {
///     field: T,
///     // Use the lifetimes in this type, or they will be unbound.
///     phantom: std::marker::PhantomData<&'w &'s ()>
/// }
/// # fn check_always_is_system<T: SystemParam + 'static>(){
/// #    bevy_ecs::system::assert_is_system(do_thing_generically::<T>);
/// # }
/// ```
pub struct StaticSystemParam<'w, 's, P: SystemParam>(SystemParamItem<'w, 's, P>);

impl<'w, 's, P: SystemParam> Deref for StaticSystemParam<'w, 's, P> {
    type Target = SystemParamItem<'w, 's, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, P: SystemParam> DerefMut for StaticSystemParam<'w, 's, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, 's, P: SystemParam> StaticSystemParam<'w, 's, P> {
    /// Get the value of the parameter
    pub fn into_inner(self) -> SystemParamItem<'w, 's, P> {
        self.0
    }
}

// SAFETY: This doesn't add any more reads, and the delegated fetch confirms it
unsafe impl<'w, 's, P: ReadOnlySystemParam + 'static> ReadOnlySystemParam
    for StaticSystemParam<'w, 's, P>
{
}

// SAFETY: all methods are just delegated to `P`'s `SystemParam` implementation
unsafe impl<P: SystemParam + 'static> SystemParam for StaticSystemParam<'_, '_, P> {
    type State = P::State;
    type Item<'world, 'state> = StaticSystemParam<'world, 'state, P>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        P::init_state(world, system_meta)
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        // SAFETY: The caller guarantees that the provided `archetype` matches the World used to initialize `state`.
        unsafe { P::new_archetype(state, archetype, system_meta) };
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        P::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        P::queue(state, system_meta, world);
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        P::validate_param(state, system_meta, world)
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: Defer to the safety of P::SystemParam
        StaticSystemParam(unsafe { P::get_param(state, system_meta, world, change_tick) })
    }
}

// SAFETY: No world access.
unsafe impl<T: ?Sized> SystemParam for PhantomData<T> {
    type State = ();
    type Item<'world, 'state> = Self;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    #[inline]
    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        PhantomData
    }
}

// SAFETY: No world access.
unsafe impl<T: ?Sized> ReadOnlySystemParam for PhantomData<T> {}

/// A [`SystemParam`] with a type that can be configured at runtime.
///
/// To be useful, this must be configured using a [`DynParamBuilder`](crate::system::DynParamBuilder) to build the system using a [`SystemParamBuilder`](crate::prelude::SystemParamBuilder).
///
/// # Examples
///
/// ```
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # #[derive(Default, Resource)]
/// # struct B;
/// #
/// # let mut world = World::new();
/// # world.init_resource::<A>();
/// # world.init_resource::<B>();
/// #
/// // If the inner parameter doesn't require any special building, use `ParamBuilder`.
/// // Either specify the type parameter on `DynParamBuilder::new()` ...
/// let system = (DynParamBuilder::new::<Res<A>>(ParamBuilder),)
///     .build_state(&mut world)
///     .build_system(expects_res_a);
/// # world.run_system_once(system);
///
/// // ... or use a factory method on `ParamBuilder` that returns a specific type.
/// let system = (DynParamBuilder::new(ParamBuilder::resource::<A>()),)
///     .build_state(&mut world)
///     .build_system(expects_res_a);
/// # world.run_system_once(system);
///
/// fn expects_res_a(mut param: DynSystemParam) {
///     // Use the `downcast` methods to retrieve the inner parameter.
///     // They will return `None` if the type does not match.
///     assert!(param.is::<Res<A>>());
///     assert!(!param.is::<Res<B>>());
///     assert!(param.downcast_mut::<Res<B>>().is_none());
///     let res = param.downcast_mut::<Res<A>>().unwrap();
///     // The type parameter can be left out if it can be determined from use.
///     let res: Res<A> = param.downcast().unwrap();
/// }
///
/// let system = (
///     // If the inner parameter also requires building,
///     // pass the appropriate `SystemParamBuilder`.
///     DynParamBuilder::new(LocalBuilder(10usize)),
///     // `DynSystemParam` is just an ordinary `SystemParam`,
///     // and can be combined with other parameters as usual!
///     ParamBuilder::query(),
/// )
///     .build_state(&mut world)
///     .build_system(|param: DynSystemParam, query: Query<()>| {
///         let local: Local<usize> = param.downcast::<Local<usize>>().unwrap();
///         assert_eq!(*local, 10);
///     });
/// # world.run_system_once(system);
/// ```
pub struct DynSystemParam<'w, 's> {
    /// A `ParamState<T>` wrapping the state for the underlying system param.
    state: &'s mut dyn Any,
    world: UnsafeWorldCell<'w>,
    system_meta: SystemMeta,
    change_tick: Tick,
}

impl<'w, 's> DynSystemParam<'w, 's> {
    /// # Safety
    /// - `state` must be a `ParamState<T>` for some inner `T: SystemParam`.
    /// - The passed [`UnsafeWorldCell`] must have access to any world data registered
    ///   in [`init_state`](SystemParam::init_state) for the inner system param.
    /// - `world` must be the same `World` that was used to initialize
    ///   [`state`](SystemParam::init_state) for the inner system param.
    unsafe fn new(
        state: &'s mut dyn Any,
        world: UnsafeWorldCell<'w>,
        system_meta: SystemMeta,
        change_tick: Tick,
    ) -> Self {
        Self {
            state,
            world,
            system_meta,
            change_tick,
        }
    }

    /// Returns `true` if the inner system param is the same as `T`.
    pub fn is<T: SystemParam>(&self) -> bool
    // See downcast() function for an explanation of the where clause
    where
        T::Item<'static, 'static>: SystemParam<Item<'w, 's> = T> + 'static,
    {
        self.state.is::<ParamState<T::Item<'static, 'static>>>()
    }

    /// Returns the inner system param if it is the correct type.
    /// This consumes the dyn param, so the returned param can have its original world and state lifetimes.
    pub fn downcast<T: SystemParam>(self) -> Option<T>
    // See downcast() function for an explanation of the where clause
    where
        T::Item<'static, 'static>: SystemParam<Item<'w, 's> = T> + 'static,
    {
        // SAFETY:
        // - `DynSystemParam::new()` ensures `state` is a `ParamState<T>`, that the world matches,
        //   and that it has access required by the inner system param.
        // - This consumes the `DynSystemParam`, so it is the only use of `world` with this access and it is available for `'w`.
        unsafe { downcast::<T>(self.state, &self.system_meta, self.world, self.change_tick) }
    }

    /// Returns the inner system parameter if it is the correct type.
    /// This borrows the dyn param, so the returned param is only valid for the duration of that borrow.
    pub fn downcast_mut<'a, T: SystemParam>(&'a mut self) -> Option<T>
    // See downcast() function for an explanation of the where clause
    where
        T::Item<'static, 'static>: SystemParam<Item<'a, 'a> = T> + 'static,
    {
        // SAFETY:
        // - `DynSystemParam::new()` ensures `state` is a `ParamState<T>`, that the world matches,
        //   and that it has access required by the inner system param.
        // - This exclusively borrows the `DynSystemParam` for `'_`, so it is the only use of `world` with this access for `'_`.
        unsafe { downcast::<T>(self.state, &self.system_meta, self.world, self.change_tick) }
    }

    /// Returns the inner system parameter if it is the correct type.
    /// This borrows the dyn param, so the returned param is only valid for the duration of that borrow,
    /// but since it only performs read access it can keep the original world lifetime.
    /// This can be useful with methods like [`Query::iter_inner()`] or [`Res::into_inner()`]
    /// to obtain references with the original world lifetime.
    pub fn downcast_mut_inner<'a, T: ReadOnlySystemParam>(&'a mut self) -> Option<T>
    // See downcast() function for an explanation of the where clause
    where
        T::Item<'static, 'static>: SystemParam<Item<'w, 'a> = T> + 'static,
    {
        // SAFETY:
        // - `DynSystemParam::new()` ensures `state` is a `ParamState<T>`, that the world matches,
        //   and that it has access required by the inner system param.
        // - The inner system param only performs read access, so it's safe to copy that access for the full `'w` lifetime.
        unsafe { downcast::<T>(self.state, &self.system_meta, self.world, self.change_tick) }
    }
}

/// # Safety
/// - `state` must be a `ParamState<T>` for some inner `T: SystemParam`.
/// - The passed [`UnsafeWorldCell`] must have access to any world data registered
///   in [`init_state`](SystemParam::init_state) for the inner system param.
/// - `world` must be the same `World` that was used to initialize
///   [`state`](SystemParam::init_state) for the inner system param.
unsafe fn downcast<'w, 's, T: SystemParam>(
    state: &'s mut dyn Any,
    system_meta: &SystemMeta,
    world: UnsafeWorldCell<'w>,
    change_tick: Tick,
) -> Option<T>
// We need a 'static version of the SystemParam to use with `Any::downcast_mut()`,
// and we need a <'w, 's> version to actually return.
// The type parameter T must be the one we return in order to get type inference from the return value.
// So we use `T::Item<'static, 'static>` as the 'static version, and require that it be 'static.
// That means the return value will be T::Item<'static, 'static>::Item<'w, 's>,
// so we constrain that to be equal to T.
// Every actual `SystemParam` implementation has `T::Item == T` up to lifetimes,
// so they should all work with this constraint.
where
    T::Item<'static, 'static>: SystemParam<Item<'w, 's> = T> + 'static,
{
    state
        .downcast_mut::<ParamState<T::Item<'static, 'static>>>()
        .map(|state| {
            // SAFETY:
            // - The caller ensures the world has access for the underlying system param,
            //   and since the downcast succeeded, the underlying system param is T.
            // - The caller ensures the `world` matches.
            unsafe { T::Item::get_param(&mut state.0, system_meta, world, change_tick) }
        })
}

/// The [`SystemParam::State`] for a [`DynSystemParam`].
pub struct DynSystemParamState(Box<dyn DynParamState>);

impl DynSystemParamState {
    pub(crate) fn new<T: SystemParam + 'static>(state: T::State) -> Self {
        Self(Box::new(ParamState::<T>(state)))
    }
}

/// Allows a [`SystemParam::State`] to be used as a trait object for implementing [`DynSystemParam`].
trait DynParamState: Sync + Send {
    /// Casts the underlying `ParamState<T>` to an `Any` so it can be downcast.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// For the specified [`Archetype`], registers the components accessed by this [`SystemParam`] (if applicable).a
    ///
    /// # Safety
    /// `archetype` must be from the [`World`] used to initialize `state` in [`SystemParam::init_state`].
    unsafe fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta);

    /// Applies any deferred mutations stored in this [`SystemParam`]'s state.
    /// This is used to apply [`Commands`] during [`apply_deferred`](crate::prelude::apply_deferred).
    ///
    /// [`Commands`]: crate::prelude::Commands
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World);

    /// Queues any deferred mutations to be applied at the next [`apply_deferred`](crate::prelude::apply_deferred).
    fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld);

    /// Refer to [`SystemParam::validate_param`].
    ///
    /// # Safety
    /// Refer to [`SystemParam::validate_param`].
    unsafe fn validate_param(&self, system_meta: &SystemMeta, world: UnsafeWorldCell) -> bool;
}

/// A wrapper around a [`SystemParam::State`] that can be used as a trait object in a [`DynSystemParam`].
struct ParamState<T: SystemParam>(T::State);

impl<T: SystemParam + 'static> DynParamState for ParamState<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    unsafe fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
        // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
        unsafe { T::new_archetype(&mut self.0, archetype, system_meta) };
    }

    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        T::apply(&mut self.0, system_meta, world);
    }

    fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld) {
        T::queue(&mut self.0, system_meta, world);
    }

    unsafe fn validate_param(&self, system_meta: &SystemMeta, world: UnsafeWorldCell) -> bool {
        T::validate_param(&self.0, system_meta, world)
    }
}

// SAFETY: `init_state` creates a state of (), which performs no access.  The interesting safety checks are on the `SystemParamBuilder`.
unsafe impl SystemParam for DynSystemParam<'_, '_> {
    type State = DynSystemParamState;

    type Item<'world, 'state> = DynSystemParam<'world, 'state>;

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        DynSystemParamState::new::<()>(())
    }

    #[inline]
    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        state.0.validate_param(system_meta, world)
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY:
        // - `state.0` is a boxed `ParamState<T>`, and its implementation of `as_any_mut` returns `self`.
        // - The state was obtained from `SystemParamBuilder::build()`, which registers all [`World`] accesses used
        //   by [`SystemParam::get_param`] with the provided [`system_meta`](SystemMeta).
        // - The caller ensures that the provided world is the same and has the required access.
        unsafe {
            DynSystemParam::new(
                state.0.as_any_mut(),
                world,
                system_meta.clone(),
                change_tick,
            )
        }
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
        unsafe { state.0.new_archetype(archetype, system_meta) };
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        state.0.apply(system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        state.0.queue(system_meta, world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        self as bevy_ecs, // Necessary for the `SystemParam` Derive when used inside `bevy_ecs`.
        system::assert_is_system,
    };
    use core::cell::RefCell;

    // Compile test for https://github.com/bevyengine/bevy/pull/2838.
    #[test]
    fn system_param_generic_bounds() {
        #[derive(SystemParam)]
        pub struct SpecialQuery<
            'w,
            's,
            D: QueryData + Send + Sync + 'static,
            F: QueryFilter + Send + Sync + 'static = (),
        > {
            _query: Query<'w, 's, D, F>,
        }

        fn my_system(_: SpecialQuery<(), ()>) {}
        assert_is_system(my_system);
    }

    // Compile tests for https://github.com/bevyengine/bevy/pull/6694.
    #[test]
    fn system_param_flexibility() {
        #[derive(SystemParam)]
        pub struct SpecialRes<'w, T: Resource> {
            _res: Res<'w, T>,
        }

        #[derive(SystemParam)]
        pub struct SpecialLocal<'s, T: FromWorld + Send + 'static> {
            _local: Local<'s, T>,
        }

        #[derive(Resource)]
        struct R;

        fn my_system(_: SpecialRes<R>, _: SpecialLocal<u32>) {}
        assert_is_system(my_system);
    }

    #[derive(Resource)]
    pub struct R<const I: usize>;

    // Compile test for https://github.com/bevyengine/bevy/pull/7001.
    #[test]
    fn system_param_const_generics() {
        #[allow(dead_code)]
        #[derive(SystemParam)]
        pub struct ConstGenericParam<'w, const I: usize>(Res<'w, R<I>>);

        fn my_system(_: ConstGenericParam<0>, _: ConstGenericParam<1000>) {}
        assert_is_system(my_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/6867.
    #[test]
    fn system_param_field_limit() {
        #[derive(SystemParam)]
        pub struct LongParam<'w> {
            // Each field should be a distinct type so there will
            // be an error if the derive messes up the field order.
            _r0: Res<'w, R<0>>,
            _r1: Res<'w, R<1>>,
            _r2: Res<'w, R<2>>,
            _r3: Res<'w, R<3>>,
            _r4: Res<'w, R<4>>,
            _r5: Res<'w, R<5>>,
            _r6: Res<'w, R<6>>,
            _r7: Res<'w, R<7>>,
            _r8: Res<'w, R<8>>,
            _r9: Res<'w, R<9>>,
            _r10: Res<'w, R<10>>,
            _r11: Res<'w, R<11>>,
            _r12: Res<'w, R<12>>,
            _r13: Res<'w, R<13>>,
            _r14: Res<'w, R<14>>,
            _r15: Res<'w, R<15>>,
            _r16: Res<'w, R<16>>,
        }

        fn long_system(_: LongParam) {}
        assert_is_system(long_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/6919.
    // Regression test for https://github.com/bevyengine/bevy/issues/7447.
    #[test]
    fn system_param_phantom_data() {
        #[derive(SystemParam)]
        struct PhantomParam<'w, T: Resource, Marker: 'static> {
            _foo: Res<'w, T>,
            marker: PhantomData<&'w Marker>,
        }

        fn my_system(_: PhantomParam<R<0>, ()>) {}
        assert_is_system(my_system);
    }

    // Compile tests for https://github.com/bevyengine/bevy/pull/6957.
    #[test]
    fn system_param_struct_variants() {
        #[derive(SystemParam)]
        pub struct UnitParam;

        #[allow(dead_code)]
        #[derive(SystemParam)]
        pub struct TupleParam<'w, 's, R: Resource, L: FromWorld + Send + 'static>(
            Res<'w, R>,
            Local<'s, L>,
        );

        fn my_system(_: UnitParam, _: TupleParam<R<0>, u32>) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/4200.
    #[test]
    fn system_param_private_fields() {
        #[derive(Resource)]
        struct PrivateResource;

        #[allow(dead_code)]
        #[derive(SystemParam)]
        pub struct EncapsulatedParam<'w>(Res<'w, PrivateResource>);

        fn my_system(_: EncapsulatedParam) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/7103.
    #[test]
    fn system_param_where_clause() {
        #[derive(SystemParam)]
        pub struct WhereParam<'w, 's, D>
        where
            D: 'static + QueryData,
        {
            _q: Query<'w, 's, D, ()>,
        }

        fn my_system(_: WhereParam<()>) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/1727.
    #[test]
    fn system_param_name_collision() {
        #[derive(Resource)]
        pub struct FetchState;

        #[derive(SystemParam)]
        pub struct Collide<'w> {
            _x: Res<'w, FetchState>,
        }

        fn my_system(_: Collide) {}
        assert_is_system(my_system);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/8192.
    #[test]
    fn system_param_invariant_lifetime() {
        #[derive(SystemParam)]
        pub struct InvariantParam<'w, 's> {
            _set: ParamSet<'w, 's, (Query<'w, 's, ()>,)>,
        }

        fn my_system(_: InvariantParam) {}
        assert_is_system(my_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/9589.
    #[test]
    fn non_sync_local() {
        fn non_sync_system(cell: Local<RefCell<u8>>) {
            assert_eq!(*cell.borrow(), 0);
        }

        let mut world = World::new();
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems(non_sync_system);
        schedule.run(&mut world);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/10207.
    #[test]
    fn param_set_non_send_first() {
        fn non_send_param_set(mut p: ParamSet<(NonSend<*mut u8>, ())>) {
            let _ = p.p0();
            p.p1();
        }

        let mut world = World::new();
        world.insert_non_send_resource(core::ptr::null_mut::<u8>());
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems((non_send_param_set, non_send_param_set, non_send_param_set));
        schedule.run(&mut world);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/10207.
    #[test]
    fn param_set_non_send_second() {
        fn non_send_param_set(mut p: ParamSet<((), NonSendMut<*mut u8>)>) {
            p.p0();
            let _ = p.p1();
        }

        let mut world = World::new();
        world.insert_non_send_resource(core::ptr::null_mut::<u8>());
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems((non_send_param_set, non_send_param_set, non_send_param_set));
        schedule.run(&mut world);
    }

    fn _dyn_system_param_type_inference(mut p: DynSystemParam) {
        // Make sure the downcast() methods are able to infer their type parameters from the use of the return type.
        // This is just a compilation test, so there is nothing to run.
        let _query: Query<()> = p.downcast_mut().unwrap();
        let _query: Query<()> = p.downcast_mut_inner().unwrap();
        let _query: Query<()> = p.downcast().unwrap();
    }
}
