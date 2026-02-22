use crate::{
    change_detection::{CheckChangeTicks, Tick},
    error::{BevyError, Result},
    never::Never,
    prelude::FromWorld,
    query::FilteredAccessSet,
    schedule::{InternedSystemSet, SystemSet},
    system::{
        check_system_change_tick, FromInput, ReadOnlySystemParam, System, SystemIn, SystemInput,
        SystemParam, SystemParamItem,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World, WorldId},
};

use alloc::{borrow::Cow, vec, vec::Vec};
use bevy_utils::prelude::DebugName;
use core::marker::PhantomData;
use variadics_please::all_tuples;

#[cfg(feature = "trace")]
use tracing::{info_span, Span};

#[cfg(feature = "trace")]
use alloc::string::ToString as _;

use super::{
    IntoSystem, ReadOnlySystem, RunSystemError, SharedStates, SystemParamBuilder,
    SystemParamValidationError, SystemStateFlags,
};

/// The metadata of a [`System`].
#[derive(Clone)]
pub struct SystemMeta {
    pub(crate) name: DebugName,
    // NOTE: this must be kept private. making a SystemMeta non-send is irreversible to prevent
    // SystemParams from overriding each other
    flags: SystemStateFlags,
    pub(crate) last_run: Tick,
    #[cfg(feature = "trace")]
    pub(crate) system_span: Span,
    #[cfg(feature = "trace")]
    pub(crate) commands_span: Span,
}

impl SystemMeta {
    pub(crate) fn new<T>() -> Self {
        let name = DebugName::type_name::<T>();
        Self {
            // These spans are initialized during plugin build, so we set the parent to `None` to prevent
            // them from being children of the span that is measuring the plugin build time.
            #[cfg(feature = "trace")]
            system_span: info_span!(parent: None, "system", name = name.clone().to_string()),
            #[cfg(feature = "trace")]
            commands_span: info_span!(parent: None, "system_commands", name = name.clone().to_string()),
            name,
            flags: SystemStateFlags::empty(),
            last_run: Tick::new(0),
        }
    }

    /// Returns the system's name
    #[inline]
    pub fn name(&self) -> &DebugName {
        &self.name
    }

    /// Returns the system's state flags
    pub fn flags(&self) -> SystemStateFlags {
        self.flags
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
            self.system_span = info_span!(parent: None, "system", name = name);
            self.commands_span = info_span!(parent: None, "system_commands", name = name);
        }
        self.name = new_name.into();
    }

    /// Gets the last time this system was run.
    #[inline]
    pub fn get_last_run(&self) -> Tick {
        self.last_run
    }

    /// Sets the last time this system was run.
    #[inline]
    pub fn set_last_run(&mut self, last_run: Tick) {
        self.last_run = last_run;
    }

    /// Returns true if the system is [`Send`].
    #[inline]
    pub fn is_send(&self) -> bool {
        !self.flags.intersects(SystemStateFlags::NON_SEND)
    }

    /// Sets the system to be not [`Send`].
    ///
    /// This is irreversible.
    #[inline]
    pub fn set_non_send(&mut self) {
        self.flags |= SystemStateFlags::NON_SEND;
    }

    /// Returns true if the system has deferred [`SystemParam`]'s
    #[inline]
    pub fn has_deferred(&self) -> bool {
        self.flags.intersects(SystemStateFlags::DEFERRED)
    }

    /// Marks the system as having deferred buffers like [`Commands`](`super::Commands`)
    /// This lets the scheduler insert [`ApplyDeferred`](`crate::prelude::ApplyDeferred`) systems automatically.
    #[inline]
    pub fn set_has_deferred(&mut self) {
        self.flags |= SystemStateFlags::DEFERRED;
    }

    /// Mark the system to run exclusively. i.e. no other systems will run at the same time.
    pub fn set_exclusive(&mut self) {
        self.flags |= SystemStateFlags::EXCLUSIVE;
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
/// and arbitrary system parameters (like [`MessageWriter`](crate::message::MessageWriter)) can be conveniently fetched.
///
/// For an alternative approach to split mutable access to the world, see [`World::resource_scope`].
///
/// # Warning
///
/// [`SystemState`] values created can be cached to improve performance,
/// and *must* be cached and reused in order for system parameters that rely on local state to work correctly.
/// These include:
/// - [`Added`](crate::query::Added), [`Changed`](crate::query::Changed) and [`Spawned`](crate::query::Spawned) query filters
/// - [`Local`](crate::system::Local) variables that hold state
/// - [`MessageReader`](crate::message::MessageReader) system parameters, which rely on a [`Local`](crate::system::Local) to track which messages have been seen
///
/// Note that this is automatically handled for you when using a [`World::run_system`](World::run_system).
///
/// # Example
///
/// Basic usage:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// #
/// # #[derive(Message)]
/// # struct MyMessage;
/// # #[derive(Resource)]
/// # struct MyResource(u32);
/// #
/// # #[derive(Component)]
/// # struct MyComponent;
/// #
/// // Work directly on the `World`
/// let mut world = World::new();
/// world.init_resource::<Messages<MyMessage>>();
///
/// // Construct a `SystemState` struct, passing in a tuple of `SystemParam`
/// // as if you were writing an ordinary system.
/// let mut system_state: SystemState<(
///     MessageWriter<MyMessage>,
///     Option<ResMut<MyResource>>,
///     Query<&MyComponent>,
/// )> = SystemState::new(&mut world);
///
/// // Use system_state.get_mut(&mut world) and unpack your system parameters into variables!
/// // system_state.get(&world) provides read-only versions of your system parameters instead.
/// let (message_writer, maybe_resource, query) = system_state.get_mut(&mut world);
///
/// // If you are using `Commands`, you can choose when you want to apply them to the world.
/// // You need to manually call `.apply(world)` on the `SystemState` to apply them.
/// ```
/// Caching:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// # use bevy_ecs::message::Messages;
/// #
/// # #[derive(Message)]
/// # struct MyMessage;
/// #[derive(Resource)]
/// struct CachedSystemState {
///     message_state: SystemState<MessageReader<'static, 'static, MyMessage>>,
/// }
///
/// // Create and store a system state once
/// let mut world = World::new();
/// world.init_resource::<Messages<MyMessage>>();
/// let initial_state: SystemState<MessageReader<MyMessage>> = SystemState::new(&mut world);
///
/// // The system state is cached in a resource
/// world.insert_resource(CachedSystemState {
///     message_state: initial_state,
/// });
///
/// // Later, fetch the cached system state, saving on overhead
/// world.resource_scope(|world, mut cached_state: Mut<CachedSystemState>| {
///     let mut message_reader = cached_state.message_state.get_mut(world);
///
///     for message in message_reader.read() {
///         println!("Hello World!");
///     }
/// });
/// ```
/// Exclusive System:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// #
/// # #[derive(Message)]
/// # struct MyMessage;
/// #
/// fn exclusive_system(world: &mut World, system_state: &mut SystemState<MessageReader<MyMessage>>) {
///     let mut message_reader = system_state.get_mut(world);
///
///     for message in message_reader.read() {
///         println!("Hello World!");
///     }
/// }
/// ```
pub struct SystemState<Param: SystemParam + 'static> {
    meta: SystemMeta,
    param_state: Param::State,
    // NOTE `param_state` must be dropped before `shared_states`
    shared_states: SharedStates,
    world_id: WorldId,
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
            #[inline]
            pub fn build_system<
                InnerOut: IntoResult<Out>,
                Out,
                Marker,
                F: FnMut($(SystemParamItem<$param>),*) -> InnerOut
                    + SystemParamFunction<Marker, In = (), Out = InnerOut, Param = ($($param,)*)>
            >
            (
                self,
                func: F,
            ) -> FunctionSystem<Marker, (), Out, F>
            {
                self.build_any_system(func)
            }

            /// Create a [`FunctionSystem`] from a [`SystemState`].
            /// This method signature allows type inference of closure parameters for a system with input.
            /// You can use [`SystemState::build_system()`] if you have no input, or [`SystemState::build_any_system()`] if you don't need type inference.
            #[inline]
            pub fn build_system_with_input<
                InnerIn: SystemInput + FromInput<In>,
                In: SystemInput,
                InnerOut: IntoResult<Out>,
                Out,
                Marker,
                F: FnMut(InnerIn, $(SystemParamItem<$param>),*) -> InnerOut
                    + SystemParamFunction<Marker, In = InnerIn, Out = InnerOut, Param = ($($param,)*)>
            >
            (
                self,
                func: F,
            ) -> FunctionSystem<Marker, In, Out, F> {
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
    #[track_caller]
    pub fn new(world: &mut World) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        meta.last_run = world.change_tick().relative_to(Tick::MAX);
        let mut component_access_set = FilteredAccessSet::new();

        let shared_states = SharedStates::new(Param::shared(), world);
        shared_states.init_access(&mut meta, &mut component_access_set, world);

        // SAFETY: drop order is upheld by field order of `Self`
        let param_state = unsafe { Param::init_state(world, &shared_states) };
        // We need to call `init_access` to ensure there are no panics from conflicts within `Param`,
        // even though we don't use the calculated access.
        Param::init_access(&param_state, &mut meta, &mut component_access_set, world);

        Self {
            meta,
            param_state,
            shared_states,
            world_id: world.id(),
        }
    }

    /// Create a [`SystemState`] from a [`SystemParamBuilder`]
    pub(crate) fn from_builder(world: &mut World, builder: impl SystemParamBuilder<Param>) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        meta.last_run = world.change_tick().relative_to(Tick::MAX);
        let mut component_access_set = FilteredAccessSet::new();

        let shared_states = SharedStates::new(Param::shared(), world);
        shared_states.init_access(&mut meta, &mut component_access_set, world);

        // SAFETY: drop order is upheld by `SystemState::drop
        let param_state = unsafe { builder.build(world, &shared_states) };
        // We need to call `init_access` to ensure there are no panics from conflicts within `Param`,
        // even though we don't use the calculated access.
        Param::init_access(&param_state, &mut meta, &mut component_access_set, world);

        Self {
            meta,
            param_state,
            shared_states,
            world_id: world.id(),
        }
    }

    /// Create a [`FunctionSystem`] from a [`SystemState`].
    /// This method signature allows any system function, but the compiler will not perform type inference on closure parameters.
    /// You can use [`SystemState::build_system()`] or [`SystemState::build_system_with_input()`] to get type inference on parameters.
    #[inline]
    pub fn build_any_system<Marker, In, Out, F>(self, func: F) -> FunctionSystem<Marker, In, Out, F>
    where
        In: SystemInput,
        F: SystemParamFunction<Marker, In: FromInput<In>, Out: IntoResult<Out>, Param = Param>,
    {
        FunctionSystem::new(
            func,
            self.meta,
            Some(FunctionSystemState {
                param: self.param_state,
                shared_states: self.shared_states,
                world_id: self.world_id,
            }),
        )
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
        // SAFETY: Param is read-only and doesn't allow mutable access to World.
        // It also matches the World this SystemState was created with.
        unsafe { self.get_unchecked(world.as_unsafe_world_cell_readonly()) }
    }

    /// Retrieve the mutable [`SystemParam`] values.
    #[inline]
    #[track_caller]
    pub fn get_mut<'w, 's>(&'s mut self, world: &'w mut World) -> SystemParamItem<'w, 's, Param> {
        self.validate_world(world.id());
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.get_unchecked(world.as_unsafe_world_cell()) }
    }

    /// Applies all state queued up for [`SystemParam`] values. For example, this will apply commands queued up
    /// by a [`Commands`](`super::Commands`) parameter to the given [`World`].
    /// This function should be called manually after the values returned by [`SystemState::get`] and [`SystemState::get_mut`]
    /// are finished being used.
    pub fn apply(&mut self, world: &mut World) {
        self.shared_states.apply_deferred(&self.meta, world);
        Param::apply(&mut self.param_state, &self.meta, world);
    }

    /// Wrapper over [`SystemParam::validate_param`].
    ///
    /// # Safety
    ///
    /// - The passed [`UnsafeWorldCell`] must have read-only access to
    ///   world data in `component_access_set`.
    /// - `world` must be the same [`World`] that was used to initialize [`state`](SystemParam::init_state).
    pub unsafe fn validate_param(
        state: &mut Self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // SAFETY: Delegated to existing `SystemParam` implementations.
        unsafe { Param::validate_param(&mut state.param_state, &state.meta, world) }
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

    /// Retrieve the [`SystemParam`] values.
    ///
    /// # Safety
    /// This call might access any of the input parameters in a way that violates Rust's mutability rules. Make sure the data
    /// access is safe in the context of global [`World`] access. The passed-in [`World`] _must_ be the [`World`] the [`SystemState`] was
    /// created with.
    #[inline]
    #[track_caller]
    pub unsafe fn get_unchecked<'w, 's>(
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
    #[track_caller]
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
    /// For example, modifying the system state of [`ResMut`](crate::system::ResMut) will obviously create issues.
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
pub struct FunctionSystem<Marker, In, Out, F>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    #[cfg(feature = "hotpatching")]
    current_ptr: subsecond::HotFnPtr,
    state: Option<FunctionSystemState<F::Param>>,
    system_meta: SystemMeta,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn(In) -> (Marker, Out)>,
}

/// The state of a [`FunctionSystem`], which must be initialized with
/// [`System::initialize`] before the system can be run. A panic will occur if
/// the system is run without being initialized.
struct FunctionSystemState<P: SystemParam> {
    /// The cached state of the system's [`SystemParam`]s.
    param: P::State,
    // NOTE: `param` must be dropped before `shared_states`
    shared_states: SharedStates,
    /// The id of the [`World`] this system was initialized with. If the world
    /// passed to [`System::run_unsafe`] or [`System::validate_param_unsafe`] does not match
    /// this id, a panic will occur.
    world_id: WorldId,
}

impl<P: SystemParam> FunctionSystemState<P> {
    fn new(world: &mut World) -> Self {
        let shared_states = SharedStates::new(P::shared(), world);
        Self {
            // SAFETY: drop order is upheld by field order
            param: unsafe { P::init_state(world, &shared_states) },
            shared_states,
            world_id: world.id(),
        }
    }
}

impl<Marker, In, Out, F> FunctionSystem<Marker, In, Out, F>
where
    F: SystemParamFunction<Marker>,
{
    #[inline]
    fn new(func: F, system_meta: SystemMeta, state: Option<FunctionSystemState<F::Param>>) -> Self {
        Self {
            func,
            #[cfg(feature = "hotpatching")]
            current_ptr: subsecond::HotFn::current(<F as SystemParamFunction<Marker>>::run)
                .ptr_address(),
            state,
            system_meta,
            marker: PhantomData,
        }
    }

    /// Return this system with a new name.
    ///
    /// Useful to give closure systems more readable and unique names for debugging and tracing.
    pub fn with_name(mut self, new_name: impl Into<Cow<'static, str>>) -> Self {
        self.system_meta.set_name(new_name.into());
        self
    }
}

// De-initializes the cloned system.
impl<Marker, In, Out, F> Clone for FunctionSystem<Marker, In, Out, F>
where
    F: SystemParamFunction<Marker> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            #[cfg(feature = "hotpatching")]
            current_ptr: subsecond::HotFn::current(<F as SystemParamFunction<Marker>>::run)
                .ptr_address(),
            state: None,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}

/// A marker type used to distinguish regular function systems from exclusive function systems.
#[doc(hidden)]
pub struct IsFunctionSystem;

impl<Marker, In, Out, F> IntoSystem<In, Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    In: SystemInput + 'static,
    Out: 'static,
    F: SystemParamFunction<Marker, In: FromInput<In>, Out: IntoResult<Out>>,
{
    type System = FunctionSystem<Marker, In, Out, F>;
    fn into_system(func: Self) -> Self::System {
        FunctionSystem::new(func, SystemMeta::new::<F>(), None)
    }
}

/// A type that may be converted to the output of a [`System`].
/// This is used to allow systems to return either a plain value or a [`Result`].
pub trait IntoResult<Out>: Sized {
    /// Converts this type into the system output type.
    fn into_result(self) -> Result<Out, RunSystemError>;
}

impl<T> IntoResult<T> for T {
    fn into_result(self) -> Result<T, RunSystemError> {
        Ok(self)
    }
}

impl<T> IntoResult<T> for Result<T, RunSystemError> {
    fn into_result(self) -> Result<T, RunSystemError> {
        self
    }
}

impl<T> IntoResult<T> for Result<T, BevyError> {
    fn into_result(self) -> Result<T, RunSystemError> {
        Ok(self?)
    }
}

// The `!` impl can't be generic in `Out`, since that would overlap with
// `impl<T> IntoResult<T> for T` when `T` = `!`.
// Use explicit impls for `()` and `bool` so diverging functions
// can be used for systems and conditions.
impl IntoResult<()> for Never {
    fn into_result(self) -> Result<(), RunSystemError> {
        self
    }
}

impl IntoResult<bool> for Never {
    fn into_result(self) -> Result<bool, RunSystemError> {
        self
    }
}

impl<Marker, In, Out, F> FunctionSystem<Marker, In, Out, F>
where
    F: SystemParamFunction<Marker>,
{
    /// Message shown when a system isn't initialized
    // When lines get too long, rustfmt can sometimes refuse to format them.
    // Work around this by storing the message separately.
    const ERROR_UNINITIALIZED: &'static str =
        "System's state was not found. Did you forget to initialize this system before running it?";
}

impl<Marker, In, Out, F> System for FunctionSystem<Marker, In, Out, F>
where
    Marker: 'static,
    In: SystemInput + 'static,
    Out: 'static,
    F: SystemParamFunction<Marker, In: FromInput<In>, Out: IntoResult<Out>>,
{
    type In = In;
    type Out = Out;

    #[inline]
    fn name(&self) -> DebugName {
        self.system_meta.name.clone()
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        self.system_meta.flags
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        #[cfg(feature = "trace")]
        let _span_guard = self.system_meta.system_span.enter();

        let change_tick = world.increment_change_tick();

        let input = F::In::from_inner(input);

        let state = self.state.as_mut().expect(Self::ERROR_UNINITIALIZED);
        assert_eq!(state.world_id, world.id(), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");
        // SAFETY:
        // - The above assert ensures the world matches.
        // - All world accesses used by `F::Param` have been registered, so the caller
        //   will ensure that there are no data access conflicts.
        let params =
            unsafe { F::Param::get_param(&mut state.param, &self.system_meta, world, change_tick) };

        #[cfg(feature = "hotpatching")]
        let out = {
            let mut hot_fn = subsecond::HotFn::current(<F as SystemParamFunction<Marker>>::run);
            // SAFETY:
            // - pointer used to call is from the current jump table
            unsafe {
                hot_fn
                    .try_call_with_ptr(self.current_ptr, (&mut self.func, input, params))
                    .expect("Error calling hotpatched system. Run a full rebuild")
            }
        };
        #[cfg(not(feature = "hotpatching"))]
        let out = self.func.run(input, params);

        self.system_meta.last_run = change_tick;
        IntoResult::into_result(out)
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        let new = subsecond::HotFn::current(<F as SystemParamFunction<Marker>>::run).ptr_address();
        if new != self.current_ptr {
            log::debug!("system {} hotpatched", self.name());
        }
        self.current_ptr = new;
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        let state = self.state.as_mut().expect(Self::ERROR_UNINITIALIZED);
        state.shared_states.apply_deferred(&self.system_meta, world);
        F::Param::apply(&mut state.param, &self.system_meta, world);
    }

    #[inline]
    fn queue_deferred(&mut self, mut world: DeferredWorld) {
        let state = self.state.as_mut().expect(Self::ERROR_UNINITIALIZED);
        state
            .shared_states
            .queue_deferred(&self.system_meta, world.reborrow());
        F::Param::queue(&mut state.param, &self.system_meta, world);
    }

    #[inline]
    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        let state = self.state.as_mut().expect(Self::ERROR_UNINITIALIZED);
        assert_eq!(state.world_id, world.id(), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");
        // SAFETY:
        // - The above assert ensures the world matches.
        // - All world accesses used by `F::Param` have been registered, so the caller
        //   will ensure that there are no data access conflicts.
        unsafe { F::Param::validate_param(&mut state.param, &self.system_meta, world) }
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet {
        if let Some(state) = &self.state {
            assert_eq!(
                state.world_id,
                world.id(),
                "System built with a different world than the one it was added to.",
            );
        }
        let state = self
            .state
            .get_or_insert_with(|| FunctionSystemState::new(world));
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
        let mut component_access_set = FilteredAccessSet::new();
        state
            .shared_states
            .init_access(&mut self.system_meta, &mut component_access_set, world);
        F::Param::init_access(
            &state.param,
            &mut self.system_meta,
            &mut component_access_set,
            world,
        );
        component_access_set
    }

    #[inline]
    fn check_change_tick(&mut self, check: CheckChangeTicks) {
        check_system_change_tick(
            &mut self.system_meta.last_run,
            check,
            self.system_meta.name.clone(),
        );
    }

    fn default_system_sets(&self) -> Vec<InternedSystemSet> {
        let set = crate::schedule::SystemTypeSet::<F>::new();
        vec![set.intern()]
    }

    fn get_last_run(&self) -> Tick {
        self.system_meta.last_run
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system_meta.last_run = last_run;
    }
}

// SAFETY: `F`'s param is [`ReadOnlySystemParam`], so this system will only read from the world.
unsafe impl<Marker, In, Out, F> ReadOnlySystem for FunctionSystem<Marker, In, Out, F>
where
    Marker: 'static,
    In: SystemInput + 'static,
    Out: 'static,
    F: SystemParamFunction<
        Marker,
        In: FromInput<In>,
        Out: IntoResult<Out>,
        Param: ReadOnlySystemParam,
    >,
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
///     // pipe the `parse_message_system`'s output into the `filter_system`s input.
///     // Type annotations should only needed when using `StaticSystemInput` as input
///     // AND the input type isn't constrained by nearby code.
///     let mut piped_system = IntoSystem::<(), Option<usize>, _>::into_system(pipe(parse_message, filter));
///     piped_system.initialize(&mut world);
///     assert_eq!(piped_system.run((), &mut world).unwrap(), Some(42));
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
