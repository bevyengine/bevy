#![expect(
    unsafe_op_in_unsafe_fn,
    reason = "See #11590. To be removed once all applicable unsafe code has an unsafe block with a safety comment."
)]

pub use crate::change_detection::{NonSend, NonSendMut, Res, ResMut};
use crate::{
    archetype::Archetypes,
    bundle::Bundles,
    change_detection::{ComponentTicksMut, ComponentTicksRef, Tick},
    component::{ComponentId, Components},
    entity::{Entities, EntityAllocator},
    query::{
        Access, FilteredAccess, FilteredAccessSet, QueryData, QueryFilter, QuerySingleError,
        QueryState, ReadOnlyQueryData,
    },
    resource::{Resource, IS_RESOURCE},
    storage::NonSendData,
    system::{Query, Single, SystemMeta},
    world::{
        unsafe_world_cell::UnsafeWorldCell, DeferredWorld, FilteredResources, FilteredResourcesMut,
        FromWorld, World,
    },
};
use alloc::{borrow::Cow, boxed::Box, vec::Vec};
pub use bevy_ecs_macros::SystemParam;
use bevy_platform::cell::SyncCell;
use bevy_platform::collections::HashMap;
use bevy_platform::hash::NoOpHash;
use bevy_platform::sync::{PoisonError, RwLock};
use bevy_ptr::{Ptr, PtrMut, UnsafeCellDeref};
use bevy_utils::prelude::DebugName;
use bevy_utils::TypeIdMap;
use core::{
    any::Any,
    any::TypeId,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use thiserror::Error;

use super::Populated;
use variadics_please::{all_tuples, all_tuples_enumerated};

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
/// # #[derive(Component)]
/// # struct SomeComponent;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// # #[derive(Message)]
/// # struct SomeMessage;
/// # #[derive(Resource)]
/// # struct SomeOtherResource;
/// # use bevy_ecs::system::SystemParam;
/// # #[derive(SystemParam)]
/// # struct ParamsExample<'w, 's> {
/// #    query:
/// Query<'w, 's, Entity>,
/// #    query2:
/// Query<'w, 's, &'static SomeComponent>,
/// #    res:
/// Res<'w, SomeResource>,
/// #    res_mut:
/// ResMut<'w, SomeOtherResource>,
/// #    local:
/// Local<'s, u8>,
/// #    commands:
/// Commands<'w, 's>,
/// #    message_reader:
/// MessageReader<'w, 's, SomeMessage>,
/// #    message_writer:
/// MessageWriter<'w, SomeMessage>
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
/// ## Custom Validation Messages
///
/// When using the derive macro, any [`SystemParamValidationError`]s will be propagated from the sub-parameters.
/// If you want to override the error message, add a `#[system_param(validation_message = "New message")]` attribute to the parameter.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// # use bevy_ecs::system::SystemParam;
/// #
/// #[derive(SystemParam)]
/// struct MyParam<'w> {
///     #[system_param(validation_message = "Custom Message")]
///     foo: Res<'w, SomeResource>,
/// }
///
/// let mut world = World::new();
/// let err = world.run_system_cached(|param: MyParam| {}).unwrap_err();
/// let expected = "Parameter `MyParam::foo` failed validation: Custom Message";
/// # #[cfg(feature="Trace")] // Without debug_utils/debug enabled MyParam::foo is stripped and breaks the assert
/// assert!(err.to_string().contains(expected));
/// ```
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
/// - [`SystemParam::init_access`] correctly registers all [`World`] accesses used
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

    /// The vtables for all parts of the state that are shared
    fn shared() -> &'static [&'static SharedStateVTable] {
        &[]
    }

    /// Creates a new instance of this param's [`State`](SystemParam::State).
    /// # SAFETY
    /// The new state must not outlive `shared_states`.
    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State;

    /// Registers any [`World`] access used by this [`SystemParam`]
    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    );

    /// Applies any deferred mutations stored in this [`SystemParam`]'s state.
    /// This is used to apply [`Commands`] during [`ApplyDeferred`](crate::prelude::ApplyDeferred).
    ///
    /// [`Commands`]: crate::prelude::Commands
    #[inline]
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {}

    /// Queues any deferred mutations to be applied at the next [`ApplyDeferred`](crate::prelude::ApplyDeferred).
    #[inline]
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {}

    /// Validates that the param can be acquired by the [`get_param`](SystemParam::get_param).
    ///
    /// Built-in executors use this to prevent systems with invalid params from running,
    /// and any failures here will be bubbled up to the default error handler defined in [`bevy_ecs::error`],
    /// with a value of type [`SystemParamValidationError`].
    ///
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
    /// [`System::validate_param`](super::system::System::validate_param),
    /// calls this method for each supplied system param.
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have read-only access to world data
    ///   registered in [`init_access`](SystemParam::init_access).
    /// - `world` must be the same [`World`] that was used to initialize [`state`](SystemParam::init_state).
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        Ok(())
    }

    /// Creates a parameter to be passed into a [`SystemParamFunction`](super::SystemParamFunction).
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have access to any world data registered
    ///   in [`init_access`](SystemParam::init_access).
    /// - `world` must be the same [`World`] that was used to initialize [`state`](SystemParam::init_state).
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state>;
}

/// Some [`SystemParam`]s may want to share (part of) their state.
///
/// The parts of the state that are shared must implement this trait.
pub trait SystemParamSharedState: Send + Sync + 'static {
    /// Creates a new instance of the state
    fn init(world: &mut World) -> Self;

    /// Registers any [`World`] access used by this [`SystemParamSharedState`]
    fn init_access(
        &self,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    );

    /// Applies any deferred mutations stored in this state.
    #[inline]
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {}

    /// Queues any deferred mutations to be applied at the next [`ApplyDeferred`](crate::prelude::ApplyDeferred).
    #[inline]
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld) {}
}

impl<T: SystemBuffer + FromWorld + Sync> SystemParamSharedState for T {
    fn init(world: &mut World) -> Self {
        T::from_world(world)
    }

    fn init_access(
        &self,
        system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        system_meta.set_has_deferred();
    }

    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        SystemBuffer::apply(self, system_meta, world);
    }

    fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld) {
        SystemBuffer::queue(self, system_meta, world);
    }
}

/// Use this as the `SystemParam::State` for parts of the state that are shared
pub struct SharedState<S: SystemParamSharedState>(NonNull<S>);

impl<S: SystemParamSharedState> SharedState<S> {
    /// # Safety
    /// The `shared_states` must outlive the new `Self`
    pub unsafe fn new(shared_states: &SharedStates) -> Option<Self> {
        Some(Self(shared_states.get::<S>()?))
    }
}

// SAFETY: `SystemParamSharedState` is `Send`
unsafe impl<S: SystemParamSharedState> Send for SharedState<S> {}
// SAFETY: `SystemParamSharedState` is `Sync`
unsafe impl<S: SystemParamSharedState> Sync for SharedState<S> {}

impl<S: SystemParamSharedState> Deref for SharedState<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        // SAFETY: ptr always points to a valid instance
        unsafe { self.0.as_ref() }
    }
}

/// Type-erased container for all parts of a [`SystemParam`]'s state that are shared.
pub struct SharedStates(HashMap<TypeId, SharedStateData>);

impl SharedStates {
    /// Create a new [`SharedStates`] using the vtables returned by [`SystemParam::shared`]
    pub fn new(vtables: &'static [&'static SharedStateVTable], world: &mut World) -> SharedStates {
        SharedStates(
            vtables
                .iter()
                .map(|vtable| (vtable.type_id, SharedStateData::new(vtable, world)))
                .collect(),
        )
    }

    /// Get a pointer to the state if it is present.
    ///
    /// # Safety
    /// The pointer must not be used after `self` is dropped and may not mutate the state without using
    /// interior mutability.
    pub unsafe fn get<S: SystemParamSharedState>(&self) -> Option<NonNull<S>> {
        Some(self.0.get(&TypeId::of::<S>())?.ptr())
    }

    /// Registers any [`World`] access used by these [`SharedStates`]
    pub fn init_access(
        &self,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        for state in self.0.values() {
            state.init_access(system_meta, component_access_set, world);
        }
    }

    /// Apply the deferred mutations from the shared states
    pub fn apply_deferred(&mut self, system_meta: &SystemMeta, world: &mut World) {
        for state in self.0.values_mut() {
            state.apply(system_meta, world);
        }
    }

    /// Queue the deferred mutations from the shared states
    pub fn queue_deferred(&mut self, system_meta: &SystemMeta, mut world: DeferredWorld) {
        for state in self.0.values_mut() {
            state.queue(system_meta, world.reborrow());
        }
    }
}

pub(crate) struct SharedStateData {
    data: NonNull<u8>,
    vtable: &'static SharedStateVTable,
}

// SAFETY: `SystemParamSharedState` is `Send`
unsafe impl Send for SharedStateData {}
// SAFETY: `SystemParamSharedState` is `Sync`
unsafe impl Sync for SharedStateData {}

impl SharedStateData {
    fn new(vtable: &'static SharedStateVTable, world: &mut World) -> SharedStateData {
        SharedStateData {
            data: (vtable.init)(world),
            vtable,
        }
    }

    pub fn init_access(
        &self,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        // SAFETY: ptr is the correct type
        unsafe {
            (self.vtable.init_access)(
                Ptr::new(self.data),
                system_meta,
                component_access_set,
                world,
            );
        }
    }

    pub fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        // SAFETY:
        // 1. The ptr is the correct type
        // 2. We can make a `PtrMut` because we have borrowed `&mut self`
        unsafe {
            (self.vtable.apply)(PtrMut::new(self.data), system_meta, world);
        }
    }

    pub fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld) {
        // SAFETY:
        // 1. The ptr is the correct type
        // 2. We can make a `PtrMut` because we have borrowed `&mut self`
        unsafe {
            (self.vtable.queue)(PtrMut::new(self.data), system_meta, world);
        }
    }

    pub fn ptr<S: SystemParamSharedState>(&self) -> NonNull<S> {
        assert_eq!(self.vtable.type_id, TypeId::of::<S>());
        self.data.cast()
    }
}

impl Drop for SharedStateData {
    fn drop(&mut self) {
        // SAFETY: `self.data` always points to a valid instance
        unsafe {
            (self.vtable.drop)(self.data);
        }
    }
}

/// The type returned by [`SystemParam::shared`]
pub struct SharedStateVTable {
    init: fn(&mut World) -> NonNull<u8>,
    init_access: unsafe fn(
        Ptr,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ),
    apply: unsafe fn(PtrMut, &SystemMeta, &mut World),
    queue: unsafe fn(PtrMut, &SystemMeta, DeferredWorld),
    drop: unsafe fn(NonNull<u8>),
    type_id: TypeId,

    #[cfg(feature = "debug")]
    type_name: &'static str,
}

impl SharedStateVTable {
    /// Get the vtable of `S`
    pub fn of<S: SystemParamSharedState>() -> &'static Self {
        VTABLES.get_or_insert::<S>(|| {
            Box::new(SharedStateVTable {
                init: |world| {
                    let state = Box::new(S::init(world));
                    NonNull::new(Box::into_raw(state)).unwrap().cast()
                },

                init_access: |ptr, system_meta, component_access_set, world| {
                    // SAFETY: ptr is the correct type
                    let slf = unsafe { ptr.deref() };
                    S::init_access(slf, system_meta, component_access_set, world);
                },

                apply: |ptr, system_meta, world| {
                    // SAFETY: ptr is the correct type
                    let slf = unsafe { ptr.deref_mut() };
                    S::apply(slf, system_meta, world);
                },

                queue: |ptr, system_meta, world| {
                    // SAFETY: ptr is the correct type
                    let slf = unsafe { ptr.deref_mut() };
                    S::queue(slf, system_meta, world);
                },

                // SAFETY: ptr was allocated using `Box::new`
                drop: |ptr| unsafe {
                    let _ = Box::from_raw(ptr.as_ptr().cast::<S>());
                },

                type_id: TypeId::of::<S>(),

                #[cfg(feature = "debug")]
                type_name: core::any::type_name::<S>(),
            })
        })
    }
}

impl Debug for SharedStateVTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut d = f.debug_struct("SharedStateVTable");

        #[cfg(feature = "debug")]
        d.field("type_name", &self.type_name);
        #[cfg(not(feature = "debug"))]
        d.field("type_name", &"<enable debug feature to see the name>");

        d.finish_non_exhaustive()
    }
}

impl Ord for SharedStateVTable {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.type_id.cmp(&other.type_id)
    }
}

impl PartialOrd for SharedStateVTable {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for SharedStateVTable {}

impl PartialEq for SharedStateVTable {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
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

// SAFETY: Relevant query ComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Query<'_, '_, D, F> {
    type State = QueryState<D, F>;
    type Item<'w, 's> = Query<'w, 's, D, F>;

    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        QueryState::new(world)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        assert_component_access_compatibility(
            &system_meta.name,
            DebugName::type_name::<D>(),
            DebugName::type_name::<F>(),
            component_access_set,
            &state.component_access,
            world,
        );
        component_access_set.add(state.component_access.clone());
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
        // The caller ensures the world matches the one used in init_state.
        unsafe { state.query_unchecked_with_ticks(world, system_meta.last_run, change_tick) }
    }
}

fn assert_component_access_compatibility(
    system_name: &DebugName,
    query_type: DebugName,
    filter_type: DebugName,
    system_access: &FilteredAccessSet,
    current: &FilteredAccess,
    world: &World,
) {
    let conflicts = system_access.get_conflicts_single(current);
    if conflicts.is_empty() {
        return;
    }
    let mut accesses = conflicts.format_conflict_list(world);
    // Access list may be empty (if access to all components requested)
    if !accesses.is_empty() {
        accesses.push(' ');
    }
    panic!("error[B0001]: Query<{}, {}> in system {system_name} accesses component(s) {accesses}in a way that conflicts with a previous system parameter. Consider using `Without<T>` to create disjoint Queries or merging conflicting Queries into a `ParamSet`. See: https://bevy.org/learn/errors/b0001", query_type.shortname(), filter_type.shortname());
}

// SAFETY: Relevant query ComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<'a, 'b, D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for Single<'a, 'b, D, F>
{
    type State = QueryState<D, F>;
    type Item<'w, 's> = Single<'w, 's, D, F>;

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        Query::init_state(world, shared_states)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        Query::init_access(state, system_meta, component_access_set, world);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: State ensures that the components it accesses are not accessible somewhere elsewhere.
        // The caller ensures the world matches the one used in init_state.
        let query =
            unsafe { state.query_unchecked_with_ticks(world, system_meta.last_run, change_tick) };
        let single = query
            .single_inner()
            .expect("The query was expected to contain exactly one matching entity.");
        Single {
            item: single,
            _filter: PhantomData,
        }
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: State ensures that the components it accesses are not mutably accessible elsewhere
        // and the query is read only.
        // The caller ensures the world matches the one used in init_state.
        let query = unsafe {
            state.query_unchecked_with_ticks(world, system_meta.last_run, world.change_tick())
        };
        match query.single_inner() {
            Ok(_) => Ok(()),
            Err(QuerySingleError::NoEntities(_)) => Err(
                SystemParamValidationError::skipped::<Self>("No matching entities"),
            ),
            Err(QuerySingleError::MultipleEntities(_)) => Err(
                SystemParamValidationError::skipped::<Self>("Multiple matching entities"),
            ),
        }
    }
}

// SAFETY: QueryState is constrained to read-only fetches, so it only reads World.
unsafe impl<'a, 'b, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReadOnlySystemParam
    for Single<'a, 'b, D, F>
{
}

// SAFETY: Relevant query ComponentId access is applied to SystemMeta. If
// this Query conflicts with any prior access, a panic will occur.
unsafe impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam
    for Populated<'_, '_, D, F>
{
    type State = QueryState<D, F>;
    type Item<'w, 's> = Populated<'w, 's, D, F>;

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        Query::init_state(world, shared_states)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        Query::init_access(state, system_meta, component_access_set, world);
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
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY:
        // - We have read-only access to the components accessed by query.
        // - The caller ensures the world matches the one used in init_state.
        let query = unsafe {
            state.query_unchecked_with_ticks(world, system_meta.last_run, world.change_tick())
        };
        if query.is_empty() {
            Err(SystemParamValidationError::skipped::<Self>(
                "No matching entities",
            ))
        } else {
            Ok(())
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
/// # #[derive(Message)]
/// # struct MyMessage;
/// # impl MyMessage {
/// #   pub fn new() -> Self { Self }
/// # }
/// fn message_system(
///     mut set: ParamSet<(
///         // PROBLEM: `MessageReader` and `MessageWriter` cannot be used together normally,
///         // because they both need access to the same message queue.
///         // SOLUTION: `ParamSet` allows these conflicting parameters to be used safely
///         // by ensuring only one is accessed at a time.
///         MessageReader<MyMessage>,
///         MessageWriter<MyMessage>,
///         // PROBLEM: `&World` needs read access to everything, which conflicts with
///         // any mutable access in the same system.
///         // SOLUTION: `ParamSet` ensures `&World` is only accessed when we're not
///         // using the other mutable parameters.
///         &World,
///     )>,
/// ) {
///     for message in set.p0().read() {
///         // ...
///         # let _message = message;
///     }
///     set.p1().write(MyMessage::new());
///
///     let entities = set.p2().entities();
///     // ...
///     # let _entities = entities;
/// }
/// # bevy_ecs::system::assert_is_system(message_system);
/// ```
pub struct ParamSet<'w, 's, T: SystemParam> {
    param_states: &'s mut T::State,
    world: UnsafeWorldCell<'w>,
    system_meta: SystemMeta,
    change_tick: Tick,
}

macro_rules! impl_param_set {
    ($(($index: tt, $param: ident, $fn_name: ident)),*) => {
        // SAFETY: All parameters are constrained to ReadOnlySystemParam, so World is only read
        unsafe impl<'w, 's, $($param,)*> ReadOnlySystemParam for ParamSet<'w, 's, ($($param,)*)>
        where $($param: ReadOnlySystemParam,)*
        { }

        // SAFETY: Relevant parameter ComponentId access is applied to SystemMeta. If any ParamState conflicts
        // with any prior access, a panic will occur.
        unsafe impl<'_w, '_s, $($param: SystemParam,)*> SystemParam for ParamSet<'_w, '_s, ($($param,)*)>
        {
            type State = ($($param::State,)*);
            type Item<'w, 's> = ParamSet<'w, 's, ($($param,)*)>;
            fn shared() -> &'static [&'static SharedStateVTable] {
                TUPLE_VTABLES.get_or_insert::<Self::State>(|| {
                    let mut shared = Vec::new();
                    $(shared.extend($param::shared());)*

                    shared.sort_unstable();
                    shared.dedup();

                    shared
                })
            }

            #[expect(
                clippy::allow_attributes,
                reason = "This is inside a macro meant for tuples; as such, `non_snake_case` won't always lint."
            )]
            #[allow(
                non_snake_case,
                reason = "Certain variable names are provided by the caller, not by us."
            )]
            unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
                ($($param::init_state(world, shared_states),)*)
            }

            #[expect(
                clippy::allow_attributes,
                reason = "This is inside a macro meant for tuples; as such, `non_snake_case` won't always lint."
            )]
            #[allow(
                non_snake_case,
                reason = "Certain variable names are provided by the caller, not by us."
            )]
            fn init_access(state: &Self::State,  system_meta: &mut SystemMeta, component_access_set: &mut FilteredAccessSet, world: &mut World) {
                let ($($param,)*) = state;
                $(
                    // Call `init_access` on a clone of the original access set to check for conflicts
                    let component_access_set_clone = &mut component_access_set.clone();
                    $param::init_access($param, system_meta, component_access_set_clone, world);
                )*
                $(
                    // Pretend to add the param to the system alone to gather the new access,
                    // then merge its access into the system.
                    let mut access_set = FilteredAccessSet::new();
                    $param::init_access($param, system_meta, &mut access_set, world);
                    component_access_set.extend(access_set);
                )*
            }

            fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
                <($($param,)*) as SystemParam>::apply(state, system_meta, world);
            }

            fn queue(state: &mut Self::State, system_meta: &SystemMeta, mut world: DeferredWorld) {
                <($($param,)*) as SystemParam>::queue(state, system_meta, world.reborrow());
            }

            #[inline]
            unsafe fn validate_param<'w, 's>(
                state: &'s mut Self::State,
                system_meta: &SystemMeta,
                world: UnsafeWorldCell<'w>,
            ) -> Result<(), SystemParamValidationError> {
                // SAFETY: Upheld by caller
                unsafe {
                    <($($param,)*) as SystemParam>::validate_param(state, system_meta, world)
                }
            }

            #[inline]
            unsafe fn get_param<'w, 's>(
                state: &'s mut Self::State,
                system_meta: &SystemMeta,
                world: UnsafeWorldCell<'w>,
                change_tick: Tick,
            ) -> Self::Item<'w, 's> {
                ParamSet {
                    param_states: state,
                    system_meta: system_meta.clone(),
                    world,
                    change_tick,
                }
            }
        }

        impl<'w, 's, $($param: SystemParam,)*> ParamSet<'w, 's, ($($param,)*)>
        {
            $(
                /// Gets exclusive access to the parameter at index
                #[doc = stringify!($index)]
                /// in this [`ParamSet`].
                /// No other parameters may be accessed while this one is active.
                pub fn $fn_name<'a>(&'a mut self) -> SystemParamItem<'a, 'a, $param> {
                    // SAFETY: systems run without conflicts with other systems.
                    // Conflicting params in ParamSet are not accessible at the same time
                    // ParamSets are guaranteed to not conflict with other SystemParams
                    unsafe {
                        $param::get_param(
                            &mut self.param_states.$index,
                            &self.system_meta,
                            self.world,
                            self.change_tick,
                        )
                    }
                }
            )*
        }
    }
}

all_tuples_enumerated!(impl_param_set, 1, 8, P, p);

static VTABLES: StaticPerType<Box<SharedStateVTable>> = StaticPerType::new();
static TUPLE_VTABLES: StaticPerType<Vec<&'static SharedStateVTable>> = StaticPerType::new();

/// Adapted from [`bevy_reflect::utility::GenericTypeCell`]
struct StaticPerType<L: Leaky>(
    RwLock<TypeIdMap<&'static L::Leaked>>,
    PhantomData<fn() -> L>,
);

impl<L: Leaky> StaticPerType<L> {
    const fn new() -> Self {
        Self(RwLock::new(TypeIdMap::with_hasher(NoOpHash)), PhantomData)
    }

    fn get_or_insert<G>(&self, f: fn() -> L) -> &'static L::Leaked
    where
        G: Any + ?Sized,
    {
        self.get_or_insert_by_type_id(TypeId::of::<G>(), f)
    }

    fn get_by_type_id(&self, type_id: TypeId) -> Option<&'static L::Leaked> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&type_id)
            .copied()
    }

    fn get_or_insert_by_type_id(&self, type_id: TypeId, f: fn() -> L) -> &'static L::Leaked {
        match self.get_by_type_id(type_id) {
            Some(info) => info,
            None => self.insert_by_type_id(type_id, f()),
        }
    }

    fn insert_by_type_id(&self, type_id: TypeId, value: L) -> &'static L::Leaked {
        let mut write_lock = self.0.write().unwrap_or_else(PoisonError::into_inner);

        write_lock
            .entry(type_id)
            .insert({
                // We leak here in order to obtain a `&'static` reference.
                // Otherwise, we won't be able to return a reference due to the `RwLock`.
                // This should be okay, though, since we expect it to remain statically
                // available over the course of the application.
                value.leak()
            })
            .get()
    }
}

trait Leaky {
    type Leaked: ?Sized + 'static;

    fn leak(self) -> &'static Self::Leaked;
}

impl<T: 'static> Leaky for Box<T> {
    type Leaked = T;

    fn leak(self) -> &'static Self::Leaked {
        Box::leak(self)
    }
}

impl<T: 'static> Leaky for Vec<T> {
    type Leaked = [T];

    fn leak(self) -> &'static Self::Leaked {
        Vec::leak(self)
    }
}

// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for Res<'a, T> {}

// SAFETY: Res ComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for Res<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = Res<'w, T>;

    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        world.components_registrator().register_component::<T>()
    }

    fn init_access(
        &component_id: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        let combined_access = component_access_set.combined_access();
        assert!(
            !combined_access.has_resource_write(component_id),
            "error[B0002]: Res<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
            DebugName::type_name::<T>(),
            system_meta.name,
        );

        let mut filter = FilteredAccess::default();
        filter.add_component_read(component_id);
        filter.add_resource_read(component_id);
        filter.and_with(IS_RESOURCE);

        assert!(component_access_set
            .get_conflicts_single(&filter)
            .is_empty(),
            "error[B0002]: Res<{}> in system {} conflicts with a previous query. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
            DebugName::type_name::<T>(),
            system_meta.name
        );

        component_access_set.add(filter);
    }

    #[inline]
    unsafe fn validate_param(
        &mut component_id: &mut Self::State,

        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to the resource
        if let Some(entity) = unsafe { world.resource_entities() }.get(component_id)
            && let Ok(entity_ref) = world.get_entity(*entity)
            && entity_ref.contains_id(component_id)
        {
            Ok(())
        } else {
            Err(SystemParamValidationError::invalid::<Self>(
                "Resource does not exist",
            ))
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,

        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_resource_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by {} does not exist: {}",
                    system_meta.name,
                    DebugName::type_name::<T>()
                );
            });
        Res {
            value: ptr.deref(),
            ticks: ComponentTicksRef {
                added: ticks.added.deref(),
                changed: ticks.changed.deref(),
                changed_by: ticks.changed_by.map(|changed_by| changed_by.deref()),
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
        }
    }
}

// SAFETY: Res ComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: Resource> SystemParam for ResMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = ResMut<'w, T>;

    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        world.components_registrator().register_component::<T>()
    }

    fn init_access(
        &component_id: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        let combined_access = component_access_set.combined_access();
        if combined_access.has_resource_write(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous ResMut<{0}> access. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
                DebugName::type_name::<T>(), system_meta.name);
        } else if combined_access.has_resource_read(component_id) {
            panic!(
                "error[B0002]: ResMut<{}> in system {} conflicts with a previous Res<{0}> access. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
                DebugName::type_name::<T>(), system_meta.name);
        }

        let mut filter = FilteredAccess::default();
        filter.add_component_write(component_id);
        filter.add_resource_write(component_id);
        filter.and_with(IS_RESOURCE);

        assert!(component_access_set
            .get_conflicts_single(&filter)
            .is_empty(),
            "error[B0002]: ResMut<{}> in system {} conflicts with a previous query. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
            DebugName::type_name::<T>(),
            system_meta.name
        );

        component_access_set.add(filter);
    }

    #[inline]
    unsafe fn validate_param(
        &mut component_id: &mut Self::State,

        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to the resource.
        if let Some(entity) = unsafe { world.resource_entities() }.get(component_id)
            && let Ok(entity_ref) = world.get_entity(*entity)
            && entity_ref.contains_id(component_id)
        {
            Ok(())
        } else {
            Err(SystemParamValidationError::invalid::<Self>(
                "Resource does not exist",
            ))
        }
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
                    DebugName::type_name::<T>()
                );
            });
        ResMut {
            value: value.value.deref_mut::<T>(),
            ticks: ComponentTicksMut {
                added: value.ticks.added,
                changed: value.ticks.changed,
                changed_by: value.ticks.changed_by,
                last_run: system_meta.last_run,
                this_run: change_tick,
            },
        }
    }
}

// SAFETY: only reads world
unsafe impl<'w> ReadOnlySystemParam for &'w World {}

// SAFETY: `read_all` access is set and conflicts result in a panic
unsafe impl SystemParam for &'_ World {
    type State = ();
    type Item<'w, 's> = &'w World;

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {}

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        let mut filtered_access = FilteredAccess::default();

        filtered_access.read_all();
        if !component_access_set
            .get_conflicts_single(&filtered_access)
            .is_empty()
        {
            panic!("&World conflicts with a previous mutable system parameter. Allowing this would break Rust's mutability rules");
        }
        component_access_set.add(filtered_access);
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

// SAFETY: `DeferredWorld` can read all components and resources but cannot be used to gain any other mutable references.
unsafe impl<'w> SystemParam for DeferredWorld<'w> {
    type State = ();
    type Item<'world, 'state> = DeferredWorld<'world>;

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {}

    fn init_access(
        _state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        assert!(
            !component_access_set.combined_access().has_any_read(),
            "DeferredWorld in system {} conflicts with a previous access.",
            system_meta.name,
        );
        component_access_set.write_all();
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: Upheld by caller
        unsafe { world.into_deferred() }
    }
}

/// A [`SystemParam`] that provides a system-private value of `T` that persists across system calls.
///
/// The initial value is created by calling `T`'s [`FromWorld::from_world`] (or [`Default::default`] if `T: Default`).
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
/// fn counter(mut count: Local<u32>) -> u32 {
///     *count += 1;
///     *count
/// }
/// let mut counter_system = IntoSystem::into_system(counter);
/// counter_system.initialize(world);
///
/// // Counter is initialized to u32's default value of 0, and increases to 1 on first run.
/// assert_eq!(counter_system.run((), world).unwrap(), 1);
/// // Counter gets the same value and increases to 2 on its second call.
/// assert_eq!(counter_system.run((), world).unwrap(), 2);
/// ```
///
/// A simple way to set a different default value for a local is by wrapping the value with an Option.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let world = &mut World::default();
/// fn counter_from_10(mut count: Local<Option<u32>>) -> u32 {
///     let count = count.get_or_insert(10);
///     *count += 1;
///     *count
/// }
/// let mut counter_system = IntoSystem::into_system(counter_from_10);
/// counter_system.initialize(world);
///
/// // Counter is initialized at 10, and increases to 11 on first run.
/// assert_eq!(counter_system.run((), world).unwrap(), 11);
/// // Counter is only increased by 1 on subsequent runs.
/// assert_eq!(counter_system.run((), world).unwrap(), 12);
/// ```
///
/// A system can have multiple `Local` values with the same type, each with distinct values.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let world = &mut World::default();
/// fn double_counter(mut count: Local<u32>, mut double_count: Local<u32>) -> (u32, u32) {
///     *count += 1;
///     *double_count += 2;
///     (*count, *double_count)
/// }
/// let mut counter_system = IntoSystem::into_system(double_counter);
/// counter_system.initialize(world);
///
/// assert_eq!(counter_system.run((), world).unwrap(), (1, 2));
/// assert_eq!(counter_system.run((), world).unwrap(), (2, 4));
/// ```
///
/// This example shows that two systems using the same type for their own `Local` get distinct locals.
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
/// assert_eq!(read_system.run((), world).unwrap(), 0);
/// write_system.run((), world);
/// // The read local is still 0 due to the locals not being shared.
/// assert_eq!(read_system.run((), world).unwrap(), 0);
/// ```
///
/// You can use a `Local` to avoid reallocating memory every system call.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// fn some_system(mut vec: Local<Vec<u32>>) {
///     // Do your regular system logic, using the vec, as normal.
///
///     // At end of function, clear the vec's contents so its empty for next system call.
///     // If it's possible the capacity could get too large, you may want to check and resize that as well.
///     vec.clear();
/// }
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

    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        SyncCell::new(T::from_world(world))
    }

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
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
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        self.queue(system_meta, world.into());
    }
    /// Queues any deferred mutations to be applied at the next [`ApplyDeferred`](crate::prelude::ApplyDeferred).
    ///
    /// To queue structural changes to [`DeferredWorld`], a command queue of the [`DeferredWorld`]
    /// should be used via [`commands`](crate::world::DeferredWorld::commands).
    fn queue(&mut self, _system_meta: &SystemMeta, _world: DeferredWorld);
}

/// A [`SystemParam`] that stores a buffer which gets applied to the [`World`] during
/// [`ApplyDeferred`](crate::schedule::ApplyDeferred).
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
/// # use bevy_ecs::world::DeferredWorld;
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
/// // Uses deferred mutations to allow signaling the alarm from multiple systems in parallel.
/// #[derive(Resource, Default)]
/// struct AlarmFlag(bool);
///
/// impl AlarmFlag {
///     /// Sounds the alarm the next time buffers are applied via ApplyDeferred.
///     pub fn flag(&mut self) {
///         self.0 = true;
///     }
/// }
///
/// impl SystemBuffer for AlarmFlag {
///     // When `AlarmFlag` is used in a system, this function will get
///     // called the next time buffers are applied via ApplyDeferred.
///     fn queue(&mut self, system_meta: &SystemMeta, mut world: DeferredWorld) {
///         if self.0 {
///             world.resource_mut::<Alarm>().0 = true;
///             self.0 = false;
///         }
///     }
/// }
///
/// // Sound the alarm if there are any criminals who pose a threat.
/// fn alert_criminal(
///     settlement: Single<&Settlement>,
///     criminals: Query<&Criminal>,
///     mut alarm: Deferred<AlarmFlag>
/// ) {
///     for criminal in &criminals {
///         // Only sound the alarm if the criminal is a threat.
///         // For this example, assume that this check is expensive to run.
///         // Since the majority of this system's run-time is dominated
///         // by calling `is_threat()`, we defer sounding the alarm to
///         // allow this system to run in parallel with other alarm systems.
///         if criminal.is_threat(*settlement) {
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
    pub fn reborrow(&mut self) -> Deferred<'_, T> {
        Deferred(self.0)
    }
}

// SAFETY: Only local state is accessed.
unsafe impl<T: SystemBuffer> ReadOnlySystemParam for Deferred<'_, T> {}

// SAFETY: Only local state is accessed.
unsafe impl<T: SystemBuffer> SystemParam for Deferred<'_, T> {
    type State = SyncCell<T>;
    type Item<'w, 's> = Deferred<'s, T>;

    #[track_caller]
    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        SyncCell::new(T::from_world(world))
    }

    fn init_access(
        _state: &Self::State,
        system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        system_meta.set_has_deferred();
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

/// A dummy type to tell the executor to run the system exclusively.
pub struct ExclusiveMarker(PhantomData<()>);

// SAFETY: No world access.
unsafe impl SystemParam for ExclusiveMarker {
    type State = ();
    type Item<'w, 's> = Self;

    #[inline]
    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {}

    fn init_access(
        _state: &Self::State,
        system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        system_meta.set_exclusive();
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        Self(PhantomData)
    }
}

// SAFETY: Does not read any world state
unsafe impl ReadOnlySystemParam for ExclusiveMarker {}

/// A dummy type that is [`!Send`](Send), to force systems to run on the main thread.
pub struct NonSendMarker(PhantomData<*mut ()>);

// SAFETY: No world access.
unsafe impl SystemParam for NonSendMarker {
    type State = ();
    type Item<'w, 's> = Self;

    #[inline]
    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {}

    fn init_access(
        _state: &Self::State,
        system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        system_meta.set_non_send();
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,

        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        Self(PhantomData)
    }
}

// SAFETY: Does not read any world state
unsafe impl ReadOnlySystemParam for NonSendMarker {}

// SAFETY: Only reads a single World non-send resource
unsafe impl<'w, T> ReadOnlySystemParam for NonSend<'w, T> {}

// SAFETY: NonSendComponentId access is applied to SystemMeta. If this
// NonSend conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSend<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSend<'w, T>;

    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        world.components_registrator().register_non_send::<T>()
    }

    fn init_access(
        &component_id: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        system_meta.set_non_send();

        let combined_access = component_access_set.combined_access();
        assert!(
            !combined_access.has_resource_write(component_id),
            "error[B0002]: NonSend<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
            DebugName::type_name::<T>(),
            system_meta.name,
        );
        component_access_set.add_unfiltered_resource_read(component_id);
    }

    #[inline]
    unsafe fn validate_param(
        &mut component_id: &mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to non-send metadata.
        if unsafe { world.storages() }
            .non_sends
            .get(component_id)
            .is_some_and(NonSendData::is_present)
        {
            Ok(())
        } else {
            Err(SystemParamValidationError::invalid::<Self>(
                "Non-send resource does not exist",
            ))
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_non_send_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    DebugName::type_name::<T>()
                );
            });
        NonSend {
            value: ptr.deref(),
            ticks: ComponentTicksRef::from_tick_cells(ticks, system_meta.last_run, change_tick),
        }
    }
}

// SAFETY: NonSendMut ComponentId access is applied to SystemMeta. If this
// NonSendMut conflicts with any prior access, a panic will occur.
unsafe impl<'a, T: 'static> SystemParam for NonSendMut<'a, T> {
    type State = ComponentId;
    type Item<'w, 's> = NonSendMut<'w, T>;

    unsafe fn init_state(world: &mut World, _shared_states: &SharedStates) -> Self::State {
        world.components_registrator().register_non_send::<T>()
    }

    fn init_access(
        &component_id: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
        system_meta.set_non_send();

        let combined_access = component_access_set.combined_access();
        if combined_access.has_resource_write(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous mutable resource access ({0}). Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
                DebugName::type_name::<T>(), system_meta.name);
        } else if combined_access.has_resource_read(component_id) {
            panic!(
                "error[B0002]: NonSendMut<{}> in system {} conflicts with a previous immutable resource access ({0}). Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002",
                DebugName::type_name::<T>(), system_meta.name);
        }
        component_access_set.add_unfiltered_resource_write(component_id);
    }

    #[inline]
    unsafe fn validate_param(
        &mut component_id: &mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Read-only access to non-send metadata.
        if unsafe { world.storages() }
            .non_sends
            .get(component_id)
            .is_some_and(NonSendData::is_present)
        {
            Ok(())
        } else {
            Err(SystemParamValidationError::invalid::<Self>(
                "Non-send resource does not exist",
            ))
        }
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        &mut component_id: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        let (ptr, ticks) = world
            .get_non_send_with_ticks(component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Non-send resource requested by {} does not exist: {}",
                    system_meta.name,
                    DebugName::type_name::<T>()
                );
            });
        NonSendMut {
            value: ptr.assert_unique().deref_mut(),
            ticks: ComponentTicksMut::from_tick_cells(ticks, system_meta.last_run, change_tick),
        }
    }
}

// SAFETY: Only reads World archetypes
unsafe impl<'a> ReadOnlySystemParam for &'a Archetypes {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Archetypes {
    type State = ();
    type Item<'w, 's> = &'w Archetypes;

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

        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.entities()
    }
}

// SAFETY: Only reads World entities
unsafe impl<'a> ReadOnlySystemParam for &'a EntityAllocator {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a EntityAllocator {
    type State = ();
    type Item<'w, 's> = &'w EntityAllocator;

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

        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        world.entity_allocator()
    }
}

// SAFETY: Only reads World bundles
unsafe impl<'a> ReadOnlySystemParam for &'a Bundles {}

// SAFETY: no component value access
unsafe impl<'a> SystemParam for &'a Bundles {
    type State = ();
    type Item<'w, 's> = &'w Bundles;

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
#[derive(Debug, Clone, Copy)]
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
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        SystemChangeTick {
            last_run: system_meta.last_run,
            this_run: change_tick,
        }
    }
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: SystemParam> SystemParam for Option<T> {
    type State = T::State;

    type Item<'world, 'state> = Option<T::Item<'world, 'state>>;

    fn shared() -> &'static [&'static SharedStateVTable] {
        T::shared()
    }

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        T::init_state(world, shared_states)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        T::init_access(state, system_meta, component_access_set, world);
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: Upheld by caller
        unsafe {
            T::validate_param(state, system_meta, world)
                .ok()
                .map(|()| T::get_param(state, system_meta, world, change_tick))
        }
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        T::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        T::queue(state, system_meta, world);
    }
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: ReadOnlySystemParam> ReadOnlySystemParam for Option<T> {}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: SystemParam> SystemParam for Result<T, SystemParamValidationError> {
    type State = T::State;

    type Item<'world, 'state> = Result<T::Item<'world, 'state>, SystemParamValidationError>;

    fn shared() -> &'static [&'static SharedStateVTable] {
        T::shared()
    }

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        T::init_state(world, shared_states)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        T::init_access(state, system_meta, component_access_set, world);
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: Upheld by caller
        unsafe {
            T::validate_param(state, system_meta, world)
                .map(|()| T::get_param(state, system_meta, world, change_tick))
        }
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        T::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        T::queue(state, system_meta, world);
    }
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: ReadOnlySystemParam> ReadOnlySystemParam for Result<T, SystemParamValidationError> {}

/// A [`SystemParam`] that wraps another parameter and causes its system to skip instead of failing when the parameter is invalid.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Resource)]
/// # struct SomeResource;
/// // This system will fail if `SomeResource` is not present.
/// fn fails_on_missing_resource(res: Res<SomeResource>) {}
///
/// // This system will skip without error if `SomeResource` is not present.
/// fn skips_on_missing_resource(res: If<Res<SomeResource>>) {
///     // The inner parameter is available using `Deref`
///     let some_resource: &SomeResource = &res;
/// }
/// # bevy_ecs::system::assert_is_system(skips_on_missing_resource);
/// ```
#[derive(Debug)]
pub struct If<T>(pub T);

impl<T> If<T> {
    /// Returns the inner `T`.
    ///
    /// The inner value is `pub`, so you can also obtain it by destructuring the parameter:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct SomeResource;
    /// fn skips_on_missing_resource(If(res): If<Res<SomeResource>>) {
    ///     let some_resource: Res<SomeResource> = res;
    /// }
    /// # bevy_ecs::system::assert_is_system(skips_on_missing_resource);
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for If<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for If<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: SystemParam> SystemParam for If<T> {
    type State = T::State;

    type Item<'world, 'state> = If<T::Item<'world, 'state>>;

    fn shared() -> &'static [&'static SharedStateVTable] {
        T::shared()
    }

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        T::init_state(world, shared_states)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        T::init_access(state, system_meta, component_access_set, world);
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Upheld by caller
        unsafe { T::validate_param(state, system_meta, world) }.map_err(|mut e| {
            e.skipped = true;
            e
        })
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: Upheld by caller.
        If(unsafe { T::get_param(state, system_meta, world, change_tick) })
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        T::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        T::queue(state, system_meta, world);
    }
}

// SAFETY: Delegates to `T`, which ensures the safety requirements are met
unsafe impl<T: ReadOnlySystemParam> ReadOnlySystemParam for If<T> {}

// SAFETY: Registers access for each element of `state`.
// If any one conflicts, it will panic.
unsafe impl<T: SystemParam> SystemParam for Vec<T> {
    type State = Vec<T::State>;

    type Item<'world, 'state> = Vec<T::Item<'world, 'state>>;

    fn shared() -> &'static [&'static SharedStateVTable] {
        T::shared()
    }

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
        Vec::new()
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        for state in state {
            T::init_access(state, system_meta, component_access_set, world);
        }
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        for state in state {
            // SAFETY: Upheld by caller
            unsafe { T::validate_param(state, system_meta, world)? };
        }
        Ok(())
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
            // - We initialized the access for each parameter in `init_access`, so the caller ensures we have access to any world data needed by each param.
            // - The caller ensures this was the world used to initialize our state, and we used that world to initialize parameter states
            .map(|state| unsafe { T::get_param(state, system_meta, world, change_tick) })
            .collect()
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

// SAFETY: Registers access for each element of `state`.
// If any one conflicts with a previous parameter,
// the call passing a copy of the current access will panic.
unsafe impl<T: SystemParam> SystemParam for ParamSet<'_, '_, Vec<T>> {
    type State = Vec<T::State>;

    type Item<'world, 'state> = ParamSet<'world, 'state, Vec<T>>;

    fn shared() -> &'static [&'static SharedStateVTable] {
        T::shared()
    }

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
        Vec::new()
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        for state in state {
            // Call `init_access` on a clone of the original access set to check for conflicts
            let component_access_set_clone = &mut component_access_set.clone();
            T::init_access(state, system_meta, component_access_set_clone, world);
        }
        for state in state {
            // Pretend to add the param to the system alone to gather the new access,
            // then merge its access into the system.
            let mut access_set = FilteredAccessSet::new();
            T::init_access(state, system_meta, &mut access_set, world);
            component_access_set.extend(access_set);
        }
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
        // - We initialized the access for each parameter, so the caller ensures we have access to any world data needed by any param.
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
                // - We initialized the access for each parameter, so the caller ensures we have access to any world data needed by any param.
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

        #[expect(
            clippy::allow_attributes,
            reason = "This is in a macro, and as such, the below lints may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "Certain variable names are provided by the caller, not by us."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use some of the parameters."
        )]
        #[allow(clippy::unused_unit, reason = "Zero length tuple is unit.")]
        $(#[$meta])*
        // SAFETY: implementers of each `SystemParam` in the tuple have validated their impls
        unsafe impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'w, 's> = ($($param::Item::<'w, 's>,)*);

            fn shared() -> &'static [&'static SharedStateVTable] {
                TUPLE_VTABLES.get_or_insert::<Self::State>(|| {
                    let mut shared = Vec::new();
                    $(shared.extend($param::shared());)*

                    shared.sort_unstable();
                    shared.dedup();

                    shared
                })
            }

            #[inline]
            #[track_caller]
            unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
                ($($param::init_state(world, shared_states),)*)
            }

            fn init_access(
                state: &Self::State,
                _system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
                let ($($param,)*) = state;
                $($param::init_access($param, _system_meta, _component_access_set, _world);)*
            }

            #[inline]
            fn apply(($($param,)*): &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
                $($param::apply($param, system_meta, world);)*
            }

            #[inline]
            #[allow(
                unused_mut,
                reason = "The `world` parameter is unused for zero-length tuples; however, it must be mutable for other lengths of tuples."
            )]
            fn queue(($($param,)*): &mut Self::State, system_meta: &SystemMeta, mut world: DeferredWorld) {
                $($param::queue($param, system_meta, world.reborrow());)*
            }

            #[inline]
            unsafe fn validate_param(
                state: &mut Self::State,
                system_meta: &SystemMeta,
                world: UnsafeWorldCell,
            ) -> Result<(), SystemParamValidationError> {
                let ($($param,)*) = state;

                #[allow(
                    unused_unsafe,
                    reason = "Zero-length tuples won't have any params to validate."
                )]
                // SAFETY: Upheld by caller
                unsafe {
                    $(
                        $param::validate_param($param, system_meta, world)?;
                    )*
                }
                Ok(())
            }

            #[inline]
            #[track_caller]
            unsafe fn get_param<'w, 's>(
                state: &'s mut Self::State,

                system_meta: &SystemMeta,
                world: UnsafeWorldCell<'w>,
                change_tick: Tick,
            ) -> Self::Item<'w, 's> {
                let ($($param,)*) = state;

                #[allow(
                    unused_unsafe,
                    reason = "Zero-length tuples won't have any params to validate."
                )]
                // SAFETY: Upheld by caller
                unsafe {
                    #[allow(
                        clippy::unused_unit,
                        reason = "Zero-length tuples won't have any params to get."
                    )]
                    ($($param::get_param($param, system_meta, world, change_tick),)*)
                }
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

    fn shared() -> &'static [&'static SharedStateVTable] {
        P::shared()
    }

    unsafe fn init_state(world: &mut World, shared_states: &SharedStates) -> Self::State {
        P::init_state(world, shared_states)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        P::init_access(state, system_meta, component_access_set, world);
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        P::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        P::queue(state, system_meta, world);
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Upheld by caller
        unsafe { P::validate_param(state, system_meta, world) }
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

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {}

    fn init_access(
        _state: &Self::State,
        _system_meta: &mut SystemMeta,
        _component_access_set: &mut FilteredAccessSet,
        _world: &mut World,
    ) {
    }

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
    /// - `state` must be a `ParamState<T>` for some inner `T: SystemParam` with no shared states.
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
        // TODO support shared state for `DynSystemParam`
        assert!(
            T::shared().is_empty(),
            "`DynSystemParam` must not have shared state"
        );
        Self(Box::new(ParamState::<T>(state)))
    }
}

/// Allows a [`SystemParam::State`] to be used as a trait object for implementing [`DynSystemParam`].
trait DynParamState: Sync + Send + Any {
    /// Applies any deferred mutations stored in this [`SystemParam`]'s state.
    /// This is used to apply [`Commands`] during [`ApplyDeferred`](crate::prelude::ApplyDeferred).
    ///
    /// [`Commands`]: crate::prelude::Commands
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World);

    /// Queues any deferred mutations to be applied at the next [`ApplyDeferred`](crate::prelude::ApplyDeferred).
    fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld);

    /// Registers any [`World`] access used by this [`SystemParam`]
    fn init_access(
        &self,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    );

    /// Refer to [`SystemParam::validate_param`].
    ///
    /// # Safety
    /// Refer to [`SystemParam::validate_param`].
    unsafe fn validate_param(
        &mut self,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError>;
}

/// A wrapper around a [`SystemParam::State`] that can be used as a trait object in a [`DynSystemParam`].
struct ParamState<T: SystemParam>(T::State);

impl<T: SystemParam + 'static> DynParamState for ParamState<T> {
    fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
        T::apply(&mut self.0, system_meta, world);
    }

    fn queue(&mut self, system_meta: &SystemMeta, world: DeferredWorld) {
        T::queue(&mut self.0, system_meta, world);
    }

    fn init_access(
        &self,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        T::init_access(&self.0, system_meta, component_access_set, world);
    }

    unsafe fn validate_param(
        &mut self,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Upheld by caller
        unsafe { T::validate_param(&mut self.0, system_meta, world) }
    }
}

// SAFETY: Delegates to the wrapped parameter, which ensures the safety requirements are met
unsafe impl SystemParam for DynSystemParam<'_, '_> {
    type State = DynSystemParamState;

    type Item<'world, 'state> = DynSystemParam<'world, 'state>;

    fn shared() -> &'static [&'static SharedStateVTable] {
        &[]
    }

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
        DynSystemParamState::new::<()>(())
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        state
            .0
            .init_access(system_meta, component_access_set, world);
    }

    #[inline]
    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Upheld by caller.
        unsafe { state.0.validate_param(system_meta, world) }
    }

    #[inline]
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY:
        // - `state.0` is a boxed `ParamState<T>`.
        // - `init_access` calls `DynParamState::init_access`, which calls `init_access` on the inner parameter,
        //   so the caller ensures the world has the necessary access.
        // - The caller ensures that the provided world is the same and has the required access.
        unsafe { DynSystemParam::new(state.0.as_mut(), world, system_meta.clone(), change_tick) }
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        state.0.apply(system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: DeferredWorld) {
        state.0.queue(system_meta, world);
    }
}

// SAFETY: Resource ComponentId access is applied to the access. If this FilteredResources
// conflicts with any prior access, a panic will occur.
unsafe impl SystemParam for FilteredResources<'_, '_> {
    type State = Access;

    type Item<'world, 'state> = FilteredResources<'world, 'state>;

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
        Access::new()
    }

    fn init_access(
        access: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        let combined_access = component_access_set.combined_access();
        let conflicts = combined_access.get_conflicts(access);
        if !conflicts.is_empty() {
            let accesses = conflicts.format_conflict_list(world);
            let system_name = &system_meta.name;
            panic!("error[B0002]: FilteredResources in system {system_name} accesses resources(s){accesses} in a way that conflicts with a previous system parameter. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002");
        }

        if access.has_read_all_resources() {
            component_access_set.add_unfiltered_read_all_resources();
        } else {
            for component_id in access.resource_reads_and_writes() {
                component_access_set.add_unfiltered_resource_read(component_id);
            }
        }
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: The caller ensures that `world` has access to anything registered in `init_access`,
        // and we registered all resource access in `state``.
        unsafe { FilteredResources::new(world, state, system_meta.last_run, change_tick) }
    }
}

// SAFETY: FilteredResources only reads resources.
unsafe impl ReadOnlySystemParam for FilteredResources<'_, '_> {}

// SAFETY: Resource ComponentId access is applied to the access. If this FilteredResourcesMut
// conflicts with any prior access, a panic will occur.
unsafe impl SystemParam for FilteredResourcesMut<'_, '_> {
    type State = Access;

    type Item<'world, 'state> = FilteredResourcesMut<'world, 'state>;

    unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
        Access::new()
    }

    fn init_access(
        access: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
        let combined_access = component_access_set.combined_access();
        let conflicts = combined_access.get_conflicts(access);
        if !conflicts.is_empty() {
            let accesses = conflicts.format_conflict_list(world);
            let system_name = &system_meta.name;
            panic!("error[B0002]: FilteredResourcesMut in system {system_name} accesses resources(s){accesses} in a way that conflicts with a previous system parameter. Consider removing the duplicate access. See: https://bevy.org/learn/errors/b0002");
        }

        if access.has_read_all_resources() {
            component_access_set.add_unfiltered_read_all_resources();
        } else {
            for component_id in access.resource_reads() {
                component_access_set.add_unfiltered_resource_read(component_id);
            }
        }

        if access.has_write_all_resources() {
            component_access_set.add_unfiltered_write_all_resources();
        } else {
            for component_id in access.resource_writes() {
                component_access_set.add_unfiltered_resource_write(component_id);
            }
        }
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        // SAFETY: The caller ensures that `world` has access to anything registered in `init_access`,
        // and we registered all resource access in `state``.
        unsafe { FilteredResourcesMut::new(world, state, system_meta.last_run, change_tick) }
    }
}

/// An error that occurs when a system parameter is not valid,
/// used by system executors to determine what to do with a system.
///
/// Returned as an error from [`SystemParam::validate_param`],
/// and handled using the unified error handling mechanisms defined in [`bevy_ecs::error`].
#[derive(Debug, PartialEq, Eq, Clone, Error)]
pub struct SystemParamValidationError {
    /// Whether the system should be skipped.
    ///
    /// If `false`, the error should be handled.
    /// By default, this will result in a panic. See [`error`](`crate::error`) for more information.
    ///
    /// This is the default behavior, and is suitable for system params that should *always* be valid,
    /// either because sensible fallback behavior exists (like [`Query`]) or because
    /// failures in validation should be considered a bug in the user's logic that must be immediately addressed (like [`Res`]).
    ///
    /// If `true`, the system should be skipped.
    /// This is set by wrapping the system param in [`If`],
    /// and indicates that the system is intended to only operate in certain application states.
    pub skipped: bool,

    /// A message describing the validation error.
    pub message: Cow<'static, str>,

    /// A string identifying the invalid parameter.
    /// This is usually the type name of the parameter.
    pub param: DebugName,

    /// A string identifying the field within a parameter using `#[derive(SystemParam)]`.
    /// This will be an empty string for other parameters.
    ///
    /// This will be printed after `param` in the `Display` impl, and should include a `::` prefix if non-empty.
    pub field: Cow<'static, str>,
}

impl SystemParamValidationError {
    /// Constructs a `SystemParamValidationError` that skips the system.
    /// The parameter name is initialized to the type name of `T`, so a `SystemParam` should usually pass `Self`.
    pub fn skipped<T>(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new::<T>(true, message, Cow::Borrowed(""))
    }

    /// Constructs a `SystemParamValidationError` for an invalid parameter that should be treated as an error.
    /// The parameter name is initialized to the type name of `T`, so a `SystemParam` should usually pass `Self`.
    pub fn invalid<T>(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new::<T>(false, message, Cow::Borrowed(""))
    }

    /// Constructs a `SystemParamValidationError` for an invalid parameter.
    /// The parameter name is initialized to the type name of `T`, so a `SystemParam` should usually pass `Self`.
    pub fn new<T>(
        skipped: bool,
        message: impl Into<Cow<'static, str>>,
        field: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            skipped,
            message: message.into(),
            param: DebugName::type_name::<T>(),
            field: field.into(),
        }
    }

    pub(crate) const EMPTY: Self = Self {
        skipped: false,
        message: Cow::Borrowed(""),
        param: DebugName::borrowed(""),
        field: Cow::Borrowed(""),
    };
}

impl Display for SystemParamValidationError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            fmt,
            "Parameter `{}{}` failed validation: {}",
            self.param.shortname(),
            self.field,
            self.message
        )?;
        if !self.skipped {
            write!(fmt, "\nIf this is an expected state, wrap the parameter in `Option<T>` and handle `None` when it happens, or wrap the parameter in `If<T>` to skip the system when it happens.")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{assert_is_system, SystemState};
    use alloc::vec;
    use bevy_platform::sync::{Arc, OnceLock};
    use core::cell::RefCell;
    use core::sync::atomic::AtomicBool;

    #[test]
    #[should_panic]
    fn non_send_alias() {
        #[derive(Resource)]
        struct A(usize);
        fn my_system(mut res0: NonSendMut<A>, mut res1: NonSendMut<A>) {
            res0.0 += 1;
            res1.0 += 1;
        }
        let mut world = World::new();
        world.insert_non_send(A(42));
        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems(my_system);
        schedule.run(&mut world);
    }

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
        #[expect(
            dead_code,
            reason = "This struct is used to ensure that const generics are supported as a SystemParam; thus, the inner value never needs to be read."
        )]
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

        #[expect(
            dead_code,
            reason = "This struct is used to ensure that tuple structs are supported as a SystemParam; thus, the inner values never need to be read."
        )]
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

        #[expect(
            dead_code,
            reason = "This struct is used to ensure that SystemParam's derive can't leak private fields; thus, the inner values never need to be read."
        )]
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
        world.insert_non_send(core::ptr::null_mut::<u8>());
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
        world.insert_non_send(core::ptr::null_mut::<u8>());
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

    #[test]
    #[should_panic]
    fn missing_resource_error() {
        #[derive(Resource)]
        pub struct MissingResource;

        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems(res_system);
        let mut world = World::new();
        schedule.run(&mut world);

        fn res_system(_: Res<MissingResource>) {}
    }

    #[test]
    #[should_panic]
    fn missing_message_error() {
        use crate::prelude::{Message, MessageReader};

        #[derive(Message)]
        pub struct MissingEvent;

        let mut schedule = crate::schedule::Schedule::default();
        schedule.add_systems(message_system);
        let mut world = World::new();
        schedule.run(&mut world);

        fn message_system(_: MessageReader<MissingEvent>) {}
    }

    #[test]
    fn vtables_are_not_all_the_same() {
        assert_ne!(
            TUPLE_VTABLES.get_or_insert::<(ParamState, ParamState)>(|| vec![
                SharedStateVTable::of::<ParamState>()
            ]),
            TUPLE_VTABLES.get_or_insert::<(OtherParamState, OtherParamState)>(|| vec![
                SharedStateVTable::of::<OtherParamState>()
            ]),
            "`TupleVTables` returns the same list of vtables for different tuples",
        );

        assert_ne!(
            <(Param, Param)>::shared(),
            <(OtherParam, OtherParam)>::shared(),
            "`<(..)>::shared` returns the same list of vtables for different tuples",
        );

        struct Param;
        struct ParamState;

        struct OtherParam;
        struct OtherParamState;

        // SAFETY: no world access
        unsafe impl SystemParam for Param {
            type State = SharedState<ParamState>;

            type Item<'world, 'state> = Self;

            fn shared() -> &'static [&'static SharedStateVTable] {
                static VTABLE: OnceLock<&'static [&'static SharedStateVTable]> = OnceLock::new();
                VTABLE.get_or_init(|| Box::leak(Box::new([SharedStateVTable::of::<ParamState>()])))
            }

            unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
                unreachable!()
            }

            fn init_access(
                _state: &Self::State,
                _system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
            }

            unsafe fn get_param<'world, 'state>(
                _state: &'state mut Self::State,
                _system_meta: &SystemMeta,
                _world: UnsafeWorldCell<'world>,
                _change_tick: Tick,
            ) -> Self::Item<'world, 'state> {
                Self
            }
        }

        impl SystemParamSharedState for ParamState {
            fn init(_world: &mut World) -> Self {
                unreachable!()
            }

            fn init_access(
                &self,
                _system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
            }
        }

        // SAFETY: no world access
        unsafe impl SystemParam for OtherParam {
            type State = (SharedState<OtherParamState>,);

            type Item<'world, 'state> = Self;

            fn shared() -> &'static [&'static SharedStateVTable] {
                static VTABLE: OnceLock<&'static [&'static SharedStateVTable]> = OnceLock::new();
                VTABLE.get_or_init(|| {
                    Box::leak(Box::new([SharedStateVTable::of::<OtherParamState>()]))
                })
            }

            unsafe fn init_state(_world: &mut World, _shared_states: &SharedStates) -> Self::State {
                unreachable!()
            }

            fn init_access(
                _state: &Self::State,
                _system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
                unreachable!()
            }

            unsafe fn get_param<'world, 'state>(
                _state: &'state mut Self::State,
                _system_meta: &SystemMeta,
                _world: UnsafeWorldCell<'world>,
                _change_tick: Tick,
            ) -> Self::Item<'world, 'state> {
                unreachable!()
            }
        }

        impl SystemParamSharedState for OtherParamState {
            fn init(_world: &mut World) -> Self {
                unreachable!()
            }

            fn init_access(
                &self,
                _system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
            }
        }
    }

    #[test]
    fn system_param_shared_state_is_shared() {
        use core::sync::atomic::Ordering;

        struct Param(Arc<AtomicBool>);

        let mut world = World::default();

        let mut state = SystemState::<(Param, Param)>::new(&mut world);
        let (param_a, param_b) = state.get_mut(&mut world);
        param_a.0.store(true, Ordering::SeqCst);
        assert!(
            param_b.0.load(Ordering::SeqCst),
            "system state should be shared in same SystemState"
        );

        world
            .run_system_cached(|param_a: Param, param_b: Param| {
                param_a.0.store(true, Ordering::SeqCst);
                assert!(
                    param_b.0.load(Ordering::SeqCst),
                    "system state should be shared in same system"
                );
            })
            .unwrap();

        struct SharedFlag {
            flag: Arc<AtomicBool>,
        }

        impl SystemParamSharedState for SharedFlag {
            fn init(_world: &mut World) -> Self {
                SharedFlag {
                    flag: Arc::new(AtomicBool::new(false)),
                }
            }

            fn init_access(
                &self,
                system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
                system_meta.set_has_deferred();
            }
        }

        // SAFETY: no world access
        unsafe impl SystemParam for Param {
            type State = SharedState<SharedFlag>;

            type Item<'world, 'state> = Self;

            fn shared() -> &'static [&'static SharedStateVTable] {
                static SHARED: OnceLock<&'static [&'static SharedStateVTable]> = OnceLock::new();
                SHARED.get_or_init(|| vec![SharedStateVTable::of::<SharedFlag>()].leak())
            }

            #[track_caller]
            unsafe fn init_state(_world: &mut World, shared_states: &SharedStates) -> Self::State {
                // SAFETY: requirements are upheld by caller
                unsafe { SharedState::new(shared_states) }
                    .expect("shared state should be initialized")
            }

            fn init_access(
                _state: &Self::State,
                _system_meta: &mut SystemMeta,
                _component_access_set: &mut FilteredAccessSet,
                _world: &mut World,
            ) {
            }

            unsafe fn get_param<'world, 'state>(
                state: &'state mut Self::State,
                _system_meta: &SystemMeta,
                _world: UnsafeWorldCell<'world>,
                _change_tick: Tick,
            ) -> Self::Item<'world, 'state> {
                Param(state.flag.clone())
            }
        }
    }
}
