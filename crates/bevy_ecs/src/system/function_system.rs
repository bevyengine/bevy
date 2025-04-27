use crate::{
    archetype::{ArchetypeComponentId, ArchetypeGeneration},
    component::{ComponentId, Tick},
    prelude::FromWorld,
    query::{Access, FilteredAccessSet},
    schedule::{InternedSystemSet, SystemSet},
    system::{
        check_system_change_tick, ReadOnlySystemParam, System, SystemIn, SystemInput, SystemParam,
        SystemParamItem,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World, WorldId},
};

use alloc::{borrow::Cow, vec, vec::Vec};
use core::marker::PhantomData;
use variadics_please::all_tuples;

#[cfg(feature = "trace")]
use tracing::{info_span, Span};

use super::{IntoSystem, ReadOnlySystem, SystemParamBuilder, SystemParamValidationError};

/// The metadata of a [`System`].
#[derive(Clone)]
pub struct SystemMeta {
    pub(crate) name: Cow<'static, str>,
    /// The set of component accesses for this system. This is used to determine
    /// - soundness issues (e.g. multiple [`SystemParam`]s mutably accessing the same component)
    /// - ambiguities in the schedule (e.g. two systems that have some sort of conflicting access)
    pub(crate) component_access_set: FilteredAccessSet<ComponentId>,
    /// This [`Access`] is used to determine which systems can run in parallel with each other
    /// in the multithreaded executor.
    ///
    /// We use a [`ArchetypeComponentId`] as it is more precise than just checking [`ComponentId`]:
    /// for example if you have one system with `Query<&mut T, With<A>>` and one system with `Query<&mut T, With<B>>`
    /// they conflict if you just look at the [`ComponentId`] of `T`; but if there are no archetypes with
    /// both `A`, `B` and `T` then in practice there's no risk of conflict. By using [`ArchetypeComponentId`]
    /// we can be more precise because we can check if the existing archetypes of the [`World`]
    /// cause a conflict
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    // NOTE: this must be kept private. making a SystemMeta non-send is irreversible to prevent
    // SystemParams from overriding each other
    is_send: bool,
    has_deferred: bool,
    pub(crate) last_run: Tick,
    #[cfg(feature = "trace")]
    pub(crate) system_span: Span,
    #[cfg(feature = "trace")]
    pub(crate) commands_span: Span,
}

impl SystemMeta {
    pub(crate) fn new<T>() -> Self {
        let name = core::any::type_name::<T>();
        Self {
            name: name.into(),
            archetype_component_access: Access::default(),
            component_access_set: FilteredAccessSet::default(),
            is_send: true,
            has_deferred: false,
            last_run: Tick::new(0),
            #[cfg(feature = "trace")]
            system_span: info_span!("system", name = name),
            #[cfg(feature = "trace")]
            commands_span: info_span!("system_commands", name = name),
        }
    }

    /// Returns the system's name
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name of this system.
    ///
    /// Useful to give closure systems more readable and unique names for debugging and tracing.
    #[inline]
    pub fn set_name(&mut self, new_name: impl Into<Cow<'static, str>>) {
        let new_name: Cow<'static, str> = new_name.into();
        #[cfg(feature = "trace")]
        {
            let name = new_name.as_ref();
            self.system_span = info_span!("system", name = name);
            self.commands_span = info_span!("system_commands", name = name);
        }
        self.name = new_name;
    }

    /// Returns true if the system is [`Send`].
    #[inline]
    pub fn is_send(&self) -> bool {
        self.is_send
    }

    /// Sets the system to be not [`Send`].
    ///
    /// This is irreversible.
    #[inline]
    pub fn set_non_send(&mut self) {
        self.is_send = false;
    }

    /// Returns true if the system has deferred [`SystemParam`]'s
    #[inline]
    pub fn has_deferred(&self) -> bool {
        self.has_deferred
    }

    /// Marks the system as having deferred buffers like [`Commands`](`super::Commands`)
    /// This lets the scheduler insert [`ApplyDeferred`](`crate::prelude::ApplyDeferred`) systems automatically.
    #[inline]
    pub fn set_has_deferred(&mut self) {
        self.has_deferred = true;
    }

    /// Archetype component access that is used to determine which systems can run in parallel with each other
    /// in the multithreaded executor.
    ///
    /// We use an [`ArchetypeComponentId`] as it is more precise than just checking [`ComponentId`]:
    /// for example if you have one system with `Query<&mut A, With<B>`, and one system with `Query<&mut A, Without<B>`,
    /// they conflict if you just look at the [`ComponentId`];
    /// but no archetype that matches the first query will match the second and vice versa,
    /// which means there's no risk of conflict.
    #[inline]
    pub fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    /// Returns a mutable reference to the [`Access`] for [`ArchetypeComponentId`].
    /// This is used to determine which systems can run in parallel with each other
    /// in the multithreaded executor.
    ///
    /// We use an [`ArchetypeComponentId`] as it is more precise than just checking [`ComponentId`]:
    /// for example if you have one system with `Query<&mut A, With<B>`, and one system with `Query<&mut A, Without<B>`,
    /// they conflict if you just look at the [`ComponentId`];
    /// but no archetype that matches the first query will match the second and vice versa,
    /// which means there's no risk of conflict.
    ///
    /// # Safety
    ///
    /// No access can be removed from the returned [`Access`].
    #[inline]
    pub unsafe fn archetype_component_access_mut(&mut self) -> &mut Access<ArchetypeComponentId> {
        &mut self.archetype_component_access
    }

    /// Returns a reference to the [`FilteredAccessSet`] for [`ComponentId`].
    /// Used to check if systems and/or system params have conflicting access.
    #[inline]
    pub fn component_access_set(&self) -> &FilteredAccessSet<ComponentId> {
        &self.component_access_set
    }

    /// Returns a mutable reference to the [`FilteredAccessSet`] for [`ComponentId`].
    /// Used internally to statically check if systems have conflicting access.
    ///
    /// # Safety
    ///
    /// No access can be removed from the returned [`FilteredAccessSet`].
    #[inline]
    pub unsafe fn component_access_set_mut(&mut self) -> &mut FilteredAccessSet<ComponentId> {
        &mut self.component_access_set
    }
}

// TODO: Actually use this in FunctionSystem. We should probably only do this once Systems are constructed using a World reference
// (to avoid the need for unwrapping to retrieve SystemMeta)
/// Holds on to persistent state required to drive [`SystemParam`] for a [`System`].
///
/// This is a powerful and convenient tool for working with exclusive world access,
/// allowing you to fetch data from the [`World`] as if you were running a [`System`].
/// However, simply calling `world::run_system(my_system)` using a [`World::run_system`](World::run_system)
/// can be significantly simpler and ensures that change detection and command flushing work as expected.
///
/// Borrow-checking is handled for you, allowing you to mutably access multiple compatible system parameters at once,
/// and arbitrary system parameters (like [`EventWriter`](crate::event::EventWriter)) can be conveniently fetched.
///
/// For an alternative approach to split mutable access to the world, see [`World::resource_scope`].
///
/// # Warning
///
/// [`SystemState`] values created can be cached to improve performance,
/// and *must* be cached and reused in order for system parameters that rely on local state to work correctly.
/// These include:
/// - [`Added`](crate::query::Added) and [`Changed`](crate::query::Changed) query filters
/// - [`Local`](crate::system::Local) variables that hold state
/// - [`EventReader`](crate::event::EventReader) system parameters, which rely on a [`Local`](crate::system::Local) to track which events have been seen
///
/// Note that this is automatically handled for you when using a [`World::run_system`](World::run_system).
///
/// # Example
///
/// Basic usage:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// # use bevy_ecs::event::Events;
/// #
/// # #[derive(Event)]
/// # struct MyEvent;
/// # #[derive(Resource)]
/// # struct MyResource(u32);
/// #
/// # #[derive(Component)]
/// # struct MyComponent;
/// #
/// // Work directly on the `World`
/// let mut world = World::new();
/// world.init_resource::<Events<MyEvent>>();
///
/// // Construct a `SystemState` struct, passing in a tuple of `SystemParam`
/// // as if you were writing an ordinary system.
/// let mut system_state: SystemState<(
///     EventWriter<MyEvent>,
///     Option<ResMut<MyResource>>,
///     Query<&MyComponent>,
/// )> = SystemState::new(&mut world);
///
/// // Use system_state.get_mut(&mut world) and unpack your system parameters into variables!
/// // system_state.get(&world) provides read-only versions of your system parameters instead.
/// let (event_writer, maybe_resource, query) = system_state.get_mut(&mut world);
///
/// // If you are using `Commands`, you can choose when you want to apply them to the world.
/// // You need to manually call `.apply(world)` on the `SystemState` to apply them.
/// ```
/// Caching:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// # use bevy_ecs::event::Events;
/// #
/// # #[derive(Event)]
/// # struct MyEvent;
/// #[derive(Resource)]
/// struct CachedSystemState {
///     event_state: SystemState<EventReader<'static, 'static, MyEvent>>,
/// }
///
/// // Create and store a system state once
/// let mut world = World::new();
/// world.init_resource::<Events<MyEvent>>();
/// let initial_state: SystemState<EventReader<MyEvent>> = SystemState::new(&mut world);
///
/// // The system state is cached in a resource
/// world.insert_resource(CachedSystemState {
///     event_state: initial_state,
/// });
///
/// // Later, fetch the cached system state, saving on overhead
/// world.resource_scope(|world, mut cached_state: Mut<CachedSystemState>| {
///     let mut event_reader = cached_state.event_state.get_mut(world);
///
///     for events in event_reader.read() {
///         println!("Hello World!");
///     }
/// });
/// ```
pub struct SystemState<Param: SystemParam + 'static> {
    meta: SystemMeta,
    param_state: Param::State,
    world_id: WorldId,
    archetype_generation: ArchetypeGeneration,
}

// Allow closure arguments to be inferred.
// For a closure to be used as a `SystemParamFunction`, it needs to be generic in any `'w` or `'s` lifetimes.
// Rust will only infer a closure to be generic over lifetimes if it's passed to a function with a Fn constraint.
// So, generate a function for each arity with an explicit `FnMut` constraint to enable higher-order lifetimes,
// along with a regular `SystemParamFunction` constraint to allow the system to be built.
macro_rules! impl_build_system {
    ($(#[$meta:meta])* $($param: ident),*) => {
        $(#[$meta])*
        impl<$($param: SystemParam),*> SystemState<($($param,)*)> {
            /// Create a [`FunctionSystem`] from a [`SystemState`].
            /// This method signature allows type inference of closure parameters for a system with no input.
            /// You can use [`SystemState::build_system_with_input()`] if you have input, or [`SystemState::build_any_system()`] if you don't need type inference.
            pub fn build_system<
                Out: 'static,
                Marker,
                F: FnMut($(SystemParamItem<$param>),*) -> Out
                    + SystemParamFunction<Marker, Param = ($($param,)*), In = (), Out = Out>
            >
            (
                self,
                func: F,
            ) -> FunctionSystem<Marker, F>
            {
                self.build_any_system(func)
            }

            /// Create a [`FunctionSystem`] from a [`SystemState`].
            /// This method signature allows type inference of closure parameters for a system with input.
            /// You can use [`SystemState::build_system()`] if you have no input, or [`SystemState::build_any_system()`] if you don't need type inference.
            pub fn build_system_with_input<
                Input: SystemInput,
                Out: 'static,
                Marker,
                F: FnMut(Input, $(SystemParamItem<$param>),*) -> Out
                    + SystemParamFunction<Marker, Param = ($($param,)*), In = Input, Out = Out>,
            >(
                self,
                func: F,
            ) -> FunctionSystem<Marker, F> {
                self.build_any_system(func)
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_build_system,
    0,
    16,
    P
);

impl<Param: SystemParam> SystemState<Param> {
    /// Creates a new [`SystemState`] with default state.
    ///
    /// ## Note
    /// For users of [`SystemState::get_manual`] or [`get_manual_mut`](SystemState::get_manual_mut):
    ///
    /// `new` does not cache any of the world's archetypes, so you must call [`SystemState::update_archetypes`]
    /// manually before calling `get_manual{_mut}`.
    pub fn new(world: &mut World) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        meta.last_run = world.change_tick().relative_to(Tick::MAX);
        let param_state = Param::init_state(world, &mut meta);
        Self {
            meta,
            param_state,
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
        }
    }

    /// Create a [`SystemState`] from a [`SystemParamBuilder`]
    pub(crate) fn from_builder(world: &mut World, builder: impl SystemParamBuilder<Param>) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        meta.last_run = world.change_tick().relative_to(Tick::MAX);
        let param_state = builder.build(world, &mut meta);
        Self {
            meta,
            param_state,
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
        }
    }

    /// Create a [`FunctionSystem`] from a [`SystemState`].
    /// This method signature allows any system function, but the compiler will not perform type inference on closure parameters.
    /// You can use [`SystemState::build_system()`] or [`SystemState::build_system_with_input()`] to get type inference on parameters.
    pub fn build_any_system<Marker, F: SystemParamFunction<Marker, Param = Param>>(
        self,
        func: F,
    ) -> FunctionSystem<Marker, F> {
        FunctionSystem {
            func,
            state: Some(FunctionSystemState {
                param: self.param_state,
                world_id: self.world_id,
            }),
            system_meta: self.meta,
            archetype_generation: self.archetype_generation,
            marker: PhantomData,
        }
    }

    /// Gets the metadata for this instance.
    #[inline]
    pub fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    /// Gets the metadata for this instance.
    #[inline]
    pub fn meta_mut(&mut self) -> &mut SystemMeta {
        &mut self.meta
    }

    /// Retrieve the [`SystemParam`] values. This can only be called when all parameters are read-only.
    #[inline]
    pub fn get<'w, 's>(&'s mut self, world: &'w World) -> SystemParamItem<'w, 's, Param>
    where
        Param: ReadOnlySystemParam,
    {
        self.validate_world(world.id());
        self.update_archetypes(world);
        // SAFETY: Param is read-only and doesn't allow mutable access to World.
        // It also matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world.as_unsafe_world_cell_readonly()) }
    }

    /// Retrieve the mutable [`SystemParam`] values.
    #[inline]
    pub fn get_mut<'w, 's>(&'s mut self, world: &'w mut World) -> SystemParamItem<'w, 's, Param> {
        self.validate_world(world.id());
        self.update_archetypes(world);
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world.as_unsafe_world_cell()) }
    }

    /// Applies all state queued up for [`SystemParam`] values. For example, this will apply commands queued up
    /// by a [`Commands`](`super::Commands`) parameter to the given [`World`].
    /// This function should be called manually after the values returned by [`SystemState::get`] and [`SystemState::get_mut`]
    /// are finished being used.
    pub fn apply(&mut self, world: &mut World) {
        Param::apply(&mut self.param_state, &self.meta, world);
    }

    /// Wrapper over [`SystemParam::validate_param`].
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have read-only access to
    ///   world data in `archetype_component_access`.
    /// - `world` must be the same [`World`] that was used to initialize [`state`](SystemParam::init_state).
    pub unsafe fn validate_param(
        state: &Self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Delegated to existing `SystemParam` implementations.
        unsafe { Param::validate_param(&state.param_state, &state.meta, world) }
    }

    /// Returns `true` if `world_id` matches the [`World`] that was used to call [`SystemState::new`].
    /// Otherwise, this returns false.
    #[inline]
    pub fn matches_world(&self, world_id: WorldId) -> bool {
        self.world_id == world_id
    }

    /// Asserts that the [`SystemState`] matches the provided world.
    #[inline]
    #[track_caller]
    fn validate_world(&self, world_id: WorldId) {
        #[inline(never)]
        #[track_caller]
        #[cold]
        fn panic_mismatched(this: WorldId, other: WorldId) -> ! {
            panic!("Encountered a mismatched World. This SystemState was created from {this:?}, but a method was called using {other:?}.");
        }

        if !self.matches_world(world_id) {
            panic_mismatched(self.world_id, world_id);
        }
    }

    /// Updates the state's internal view of the [`World`]'s archetypes. If this is not called before fetching the parameters,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if [`SystemState::get_manual`] or [`SystemState::get_manual_mut`] is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// [`SystemState::get`] or [`SystemState::get_mut`] do not need to call this as it will be automatically called for them.
    #[inline]
    pub fn update_archetypes(&mut self, world: &World) {
        self.update_archetypes_unsafe_world_cell(world.as_unsafe_world_cell_readonly());
    }

    /// Updates the state's internal view of the `world`'s archetypes. If this is not called before fetching the parameters,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if [`SystemState::get_manual`] or [`SystemState::get_manual_mut`] is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// [`SystemState::get`] or [`SystemState::get_mut`] do not need to call this as it will be automatically called for them.
    ///
    /// # Note
    ///
    /// This method only accesses world metadata.
    #[inline]
    pub fn update_archetypes_unsafe_world_cell(&mut self, world: UnsafeWorldCell) {
        assert_eq!(self.world_id, world.id(), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");

        let archetypes = world.archetypes();
        let old_generation =
            core::mem::replace(&mut self.archetype_generation, archetypes.generation());

        for archetype in &archetypes[old_generation..] {
            // SAFETY: The assertion above ensures that the param_state was initialized from `world`.
            unsafe { Param::new_archetype(&mut self.param_state, archetype, &mut self.meta) };
        }
    }

    /// Retrieve the [`SystemParam`] values. This can only be called when all parameters are read-only.
    /// This will not update the state's view of the world's archetypes automatically nor increment the
    /// world's change tick.
    ///
    /// For this to return accurate results, ensure [`SystemState::update_archetypes`] is called before this
    /// function.
    ///
    /// Users should strongly prefer to use [`SystemState::get`] over this function.
    #[inline]
    pub fn get_manual<'w, 's>(&'s mut self, world: &'w World) -> SystemParamItem<'w, 's, Param>
    where
        Param: ReadOnlySystemParam,
    {
        self.validate_world(world.id());
        let change_tick = world.read_change_tick();
        // SAFETY: Param is read-only and doesn't allow mutable access to World.
        // It also matches the World this SystemState was created with.
        unsafe { self.fetch(world.as_unsafe_world_cell_readonly(), change_tick) }
    }

    /// Retrieve the mutable [`SystemParam`] values.  This will not update the state's view of the world's archetypes
    /// automatically nor increment the world's change tick.
    ///
    /// For this to return accurate results, ensure [`SystemState::update_archetypes`] is called before this
    /// function.
    ///
    /// Users should strongly prefer to use [`SystemState::get_mut`] over this function.
    #[inline]
    pub fn get_manual_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> SystemParamItem<'w, 's, Param> {
        self.validate_world(world.id());
        let change_tick = world.change_tick();
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.fetch(world.as_unsafe_world_cell(), change_tick) }
    }

    /// Retrieve the [`SystemParam`] values. This will not update archetypes automatically.
    ///
    /// # Safety
    /// This call might access any of the input parameters in a way that violates Rust's mutability rules. Make sure the data
    /// access is safe in the context of global [`World`] access. The passed-in [`World`] _must_ be the [`World`] the [`SystemState`] was
    /// created with.
    #[inline]
    pub unsafe fn get_unchecked_manual<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
    ) -> SystemParamItem<'w, 's, Param> {
        let change_tick = world.increment_change_tick();
        // SAFETY: The invariants are upheld by the caller.
        unsafe { self.fetch(world, change_tick) }
    }

    /// # Safety
    /// This call might access any of the input parameters in a way that violates Rust's mutability rules. Make sure the data
    /// access is safe in the context of global [`World`] access. The passed-in [`World`] _must_ be the [`World`] the [`SystemState`] was
    /// created with.
    #[inline]
    unsafe fn fetch<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> SystemParamItem<'w, 's, Param> {
        // SAFETY: The invariants are upheld by the caller.
        let param =
            unsafe { Param::get_param(&mut self.param_state, &self.meta, world, change_tick) };
        self.meta.last_run = change_tick;
        param
    }

    /// Returns a reference to the current system param states.
    pub fn param_state(&self) -> &Param::State {
        &self.param_state
    }

    /// Returns a mutable reference to the current system param states.
    /// Marked as unsafe because modifying the system states may result in violation to certain
    /// assumptions made by the [`SystemParam`]. Use with care.
    ///
    /// # Safety
    /// Modifying the system param states may have unintended consequences.
    /// The param state is generally considered to be owned by the [`SystemParam`]. Modifications
    /// should respect any invariants as required by the [`SystemParam`].
    /// For example, modifying the system state of [`ResMut`](crate::system::ResMut) without also
    /// updating [`SystemMeta::component_access_set`] will obviously create issues.
    pub unsafe fn param_state_mut(&mut self) -> &mut Param::State {
        &mut self.param_state
    }
}

impl<Param: SystemParam> FromWorld for SystemState<Param> {
    fn from_world(world: &mut World) -> Self {
        Self::new(world)
    }
}

/// The [`System`] counter part of an ordinary function.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`SystemParam`]s. The output of the system becomes the functions return type, while the input
/// becomes the functions first parameter or `()` if no such parameter exists.
///
/// [`FunctionSystem`] must be `.initialized` before they can be run.
///
/// The [`Clone`] implementation for [`FunctionSystem`] returns a new instance which
/// is NOT initialized. The cloned system must also be `.initialized` before it can be run.
pub struct FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    state: Option<FunctionSystemState<F::Param>>,
    system_meta: SystemMeta,
    archetype_generation: ArchetypeGeneration,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> Marker>,
}

/// The state of a [`FunctionSystem`], which must be initialized with
/// [`System::initialize`] before the system can be run. A panic will occur if
/// the system is run without being initialized.
struct FunctionSystemState<P: SystemParam> {
    /// The cached state of the system's [`SystemParam`]s.
    param: P::State,
    /// The id of the [`World`] this system was initialized with. If the world
    /// passed to [`System::update_archetype_component_access`] does not match
    /// this id, a panic will occur.
    world_id: WorldId,
}

impl<Marker, F> FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    /// Return this system with a new name.
    ///
    /// Useful to give closure systems more readable and unique names for debugging and tracing.
    pub fn with_name(mut self, new_name: impl Into<Cow<'static, str>>) -> Self {
        self.system_meta.set_name(new_name.into());
        self
    }
}

// De-initializes the cloned system.
impl<Marker, F> Clone for FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            state: None,
            system_meta: SystemMeta::new::<F>(),
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

/// A marker type used to distinguish regular function systems from exclusive function systems.
#[doc(hidden)]
pub struct IsFunctionSystem;

impl<Marker, F> IntoSystem<F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
{
    type System = FunctionSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        FunctionSystem {
            func,
            state: None,
            system_meta: SystemMeta::new::<F>(),
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

impl<Marker, F> FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    /// Message shown when a system isn't initialized
    // When lines get too long, rustfmt can sometimes refuse to format them.
    // Work around this by storing the message separately.
    const ERROR_UNINITIALIZED: &'static str =
        "System's state was not found. Did you forget to initialize this system before running it?";
}

impl<Marker, F> System for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
{
    type In = F::In;
    type Out = F::Out;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.system_meta.name.clone()
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        self.system_meta.component_access_set.combined_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.system_meta.archetype_component_access
    }

    #[inline]
    fn is_send(&self) -> bool {
        self.system_meta.is_send
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        false
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        self.system_meta.has_deferred
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        #[cfg(feature = "trace")]
        let _span_guard = self.system_meta.system_span.enter();

        let change_tick = world.increment_change_tick();

        let param_state = &mut self.state.as_mut().expect(Self::ERROR_UNINITIALIZED).param;
        // SAFETY:
        // - The caller has invoked `update_archetype_component_access`, which will panic
        //   if the world does not match.
        // - All world accesses used by `F::Param` have been registered, so the caller
        //   will ensure that there are no data access conflicts.
        let params =
            unsafe { F::Param::get_param(param_state, &self.system_meta, world, change_tick) };
        let out = self.func.run(input, params);
        self.system_meta.last_run = change_tick;
        out
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        let param_state = &mut self.state.as_mut().expect(Self::ERROR_UNINITIALIZED).param;
        F::Param::apply(param_state, &self.system_meta, world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        let param_state = &mut self.state.as_mut().expect(Self::ERROR_UNINITIALIZED).param;
        F::Param::queue(param_state, &self.system_meta, world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        let param_state = &self.state.as_ref().expect(Self::ERROR_UNINITIALIZED).param;
        // SAFETY:
        // - The caller has invoked `update_archetype_component_access`, which will panic
        //   if the world does not match.
        // - All world accesses used by `F::Param` have been registered, so the caller
        //   will ensure that there are no data access conflicts.
        unsafe { F::Param::validate_param(param_state, &self.system_meta, world) }
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        if let Some(state) = &self.state {
            assert_eq!(
                state.world_id,
                world.id(),
                "System built with a different world than the one it was added to.",
            );
        } else {
            self.state = Some(FunctionSystemState {
                param: F::Param::init_state(world, &mut self.system_meta),
                world_id: world.id(),
            });
        }
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        let state = self.state.as_mut().expect(Self::ERROR_UNINITIALIZED);
        assert_eq!(state.world_id, world.id(), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");

        let archetypes = world.archetypes();
        let old_generation =
            core::mem::replace(&mut self.archetype_generation, archetypes.generation());

        for archetype in &archetypes[old_generation..] {
            // SAFETY: The assertion above ensures that the param_state was initialized from `world`.
            unsafe { F::Param::new_archetype(&mut state.param, archetype, &mut self.system_meta) };
        }
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: Tick) {
        check_system_change_tick(
            &mut self.system_meta.last_run,
            change_tick,
            self.system_meta.name.as_ref(),
        );
    }

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        let set = crate::schedule::SystemTypeSet::<Self>::new();
        vec![set.intern()]
    }

    fn get_last_run(&self) -> Tick {
        self.system_meta.last_run
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system_meta.last_run = last_run;
    }
}

/// SAFETY: `F`'s param is [`ReadOnlySystemParam`], so this system will only read from the world.
unsafe impl<Marker, F> ReadOnlySystem for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Param: ReadOnlySystemParam,
{
}

/// A trait implemented for all functions that can be used as [`System`]s.
///
/// This trait can be useful for making your own systems which accept other systems,
/// sometimes called higher order systems.
///
/// This should be used in combination with [`ParamSet`] when calling other systems
/// within your system.
/// Using [`ParamSet`] in this case avoids [`SystemParam`] collisions.
///
/// # Example
///
/// To create something like [`PipeSystem`], but in entirely safe code.
///
/// ```
/// use std::num::ParseIntError;
///
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::StaticSystemInput;
///
/// /// Pipe creates a new system which calls `a`, then calls `b` with the output of `a`
/// pub fn pipe<A, B, AMarker, BMarker>(
///     mut a: A,
///     mut b: B,
/// ) -> impl FnMut(StaticSystemInput<A::In>, ParamSet<(A::Param, B::Param)>) -> B::Out
/// where
///     // We need A and B to be systems, add those bounds
///     A: SystemParamFunction<AMarker>,
///     B: SystemParamFunction<BMarker>,
///     for<'a> B::In: SystemInput<Inner<'a> = A::Out>,
/// {
///     // The type of `params` is inferred based on the return of this function above
///     move |StaticSystemInput(a_in), mut params| {
///         let shared = a.run(a_in, params.p0());
///         b.run(shared, params.p1())
///     }
/// }
///
/// // Usage example for `pipe`:
/// fn main() {
///     let mut world = World::default();
///     world.insert_resource(Message("42".to_string()));
///
///     // pipe the `parse_message_system`'s output into the `filter_system`s input
///     let mut piped_system = IntoSystem::into_system(pipe(parse_message, filter));
///     piped_system.initialize(&mut world);
///     assert_eq!(piped_system.run((), &mut world), Some(42));
/// }
///
/// #[derive(Resource)]
/// struct Message(String);
///
/// fn parse_message(message: Res<Message>) -> Result<usize, ParseIntError> {
///     message.0.parse::<usize>()
/// }
///
/// fn filter(In(result): In<Result<usize, ParseIntError>>) -> Option<usize> {
///     result.ok().filter(|&n| n < 100)
/// }
/// ```
/// [`PipeSystem`]: crate::system::PipeSystem
/// [`ParamSet`]: crate::system::ParamSet
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid system",
    label = "invalid system"
)]
pub trait SystemParamFunction<Marker>: Send + Sync + 'static {
    /// The input type of this system. See [`System::In`].
    type In: SystemInput;
    /// The return type of this system. See [`System::Out`].
    type Out;

    /// The [`SystemParam`]/s used by this system to access the [`World`].
    type Param: SystemParam;

    /// Executes this system once. See [`System::run`] or [`System::run_unsafe`].
    fn run(
        &mut self,
        input: <Self::In as SystemInput>::Inner<'_>,
        param_value: SystemParamItem<Self::Param>,
    ) -> Self::Out;
}

/// A marker type used to distinguish function systems with and without input.
#[doc(hidden)]
pub struct HasSystemInput;

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is within a macro, and as such, the below lints may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "Certain variable names are provided by the caller, not by us."
        )]
        impl<Out, Func, $($param: SystemParam),*> SystemParamFunction<fn($($param,)*) -> Out> for Func
        where
            Func: Send + Sync + 'static,
            for <'a> &'a mut Func:
                FnMut($($param),*) -> Out +
                FnMut($(SystemParamItem<$param>),*) -> Out,
            Out: 'static
        {
            type In = ();
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, _input: (), param_value: SystemParamItem< ($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognize that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                fn call_inner<Out, $($param,)*>(
                    mut f: impl FnMut($($param,)*)->Out,
                    $($param: $param,)*
                )->Out{
                    f($($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, $($param),*)
            }
        }

        #[expect(
            clippy::allow_attributes,
            reason = "This is within a macro, and as such, the below lints may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "Certain variable names are provided by the caller, not by us."
        )]
        impl<In, Out, Func, $($param: SystemParam),*> SystemParamFunction<(HasSystemInput, fn(In, $($param,)*) -> Out)> for Func
        where
            Func: Send + Sync + 'static,
            for <'a> &'a mut Func:
                FnMut(In, $($param),*) -> Out +
                FnMut(In::Param<'_>, $(SystemParamItem<$param>),*) -> Out,
            In: SystemInput + 'static,
            Out: 'static
        {
            type In = In;
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, input: In::Inner<'_>, param_value: SystemParamItem< ($($param,)*)>) -> Out {
                fn call_inner<In: SystemInput, Out, $($param,)*>(
                    _: PhantomData<In>,
                    mut f: impl FnMut(In::Param<'_>, $($param,)*)->Out,
                    input: In::Inner<'_>,
                    $($param: $param,)*
                )->Out{
                    f(In::wrap(input), $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(PhantomData::<In>, self, input, $($param),*)
            }
        }
    };
}

// Note that we rely on the highest impl to be <= the highest order of the tuple impls
// of `SystemParam` created.
all_tuples!(impl_system_function, 0, 16, F);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_system_type_id_consistency() {
        fn test<T, In: SystemInput, Out, Marker>(function: T)
        where
            T: IntoSystem<In, Out, Marker> + Copy,
        {
            fn reference_system() {}

            use core::any::TypeId;

            let system = IntoSystem::into_system(function);

            assert_eq!(
                system.type_id(),
                function.system_type_id(),
                "System::type_id should be consistent with IntoSystem::system_type_id"
            );

            assert_eq!(
                system.type_id(),
                TypeId::of::<T::System>(),
                "System::type_id should be consistent with TypeId::of::<T::System>()"
            );

            assert_ne!(
                system.type_id(),
                IntoSystem::into_system(reference_system).type_id(),
                "Different systems should have different TypeIds"
            );
        }

        fn function_system() {}

        test(function_system);
    }
}
