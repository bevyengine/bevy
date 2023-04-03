use crate::{
    archetype::{ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::{ComponentId, Tick},
    prelude::FromWorld,
    query::{Access, FilteredAccessSet},
    system::{check_system_change_tick, ReadOnlySystemParam, System, SystemParam, SystemParamItem},
    world::{World, WorldId},
};

use bevy_utils::all_tuples;
use std::{any::TypeId, borrow::Cow, marker::PhantomData};

use super::ReadOnlySystem;

/// The metadata of a [`System`].
#[derive(Clone)]
pub struct SystemMeta {
    pub(crate) name: Cow<'static, str>,
    pub(crate) component_access_set: FilteredAccessSet<ComponentId>,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    // NOTE: this must be kept private. making a SystemMeta non-send is irreversible to prevent
    // SystemParams from overriding each other
    is_send: bool,
    pub(crate) last_run: Tick,
}

impl SystemMeta {
    pub(crate) fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            archetype_component_access: Access::default(),
            component_access_set: FilteredAccessSet::default(),
            is_send: true,
            last_run: Tick::new(0),
        }
    }

    /// Returns the system's name
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
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
}

// TODO: Actually use this in FunctionSystem. We should probably only do this once Systems are constructed using a World reference
// (to avoid the need for unwrapping to retrieve SystemMeta)
/// Holds on to persistent state required to drive [`SystemParam`] for a [`System`].
///
/// This is a very powerful and convenient tool for working with exclusive world access,
/// allowing you to fetch data from the [`World`] as if you were running a [`System`].
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
/// # Example
///
/// Basic usage:
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// # use bevy_ecs::event::Events;
/// #
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
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::system::SystemState;
/// # use bevy_ecs::event::Events;
/// #
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
///     for events in event_reader.iter() {
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

impl<Param: SystemParam> SystemState<Param> {
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

    #[inline]
    pub fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    /// Retrieve the [`SystemParam`] values. This can only be called when all parameters are read-only.
    #[inline]
    pub fn get<'w, 's>(&'s mut self, world: &'w World) -> SystemParamItem<'w, 's, Param>
    where
        Param: ReadOnlySystemParam,
    {
        self.validate_world(world);
        self.update_archetypes(world);
        // SAFETY: Param is read-only and doesn't allow mutable access to World. It also matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Retrieve the mutable [`SystemParam`] values.
    #[inline]
    pub fn get_mut<'w, 's>(&'s mut self, world: &'w mut World) -> SystemParamItem<'w, 's, Param> {
        self.validate_world(world);
        self.update_archetypes(world);
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Applies all state queued up for [`SystemParam`] values. For example, this will apply commands queued up
    /// by a [`Commands`](`super::Commands`) parameter to the given [`World`].
    /// This function should be called manually after the values returned by [`SystemState::get`] and [`SystemState::get_mut`]
    /// are finished being used.
    pub fn apply(&mut self, world: &mut World) {
        Param::apply(&mut self.param_state, &self.meta, world);
    }

    #[inline]
    pub fn matches_world(&self, world: &World) -> bool {
        self.world_id == world.id()
    }

    /// Asserts that the [`SystemState`] matches the provided [`World`].
    #[inline]
    fn validate_world(&self, world: &World) {
        assert!(self.matches_world(world), "Encountered a mismatched World. A SystemState cannot be used with Worlds other than the one it was created with.");
    }

    /// Updates the state's internal view of the `world`'s archetypes. If this is not called before fetching the parameters,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if [`SystemState::get_manual`] or [`SystemState::get_manual_mut`] is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// [`SystemState::get`] or [`SystemState::get_mut`] do not need to call this as it will be automatically called for them.
    #[inline]
    pub fn update_archetypes(&mut self, world: &World) {
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            Param::new_archetype(
                &mut self.param_state,
                &archetypes[ArchetypeId::new(archetype_index)],
                &mut self.meta,
            );
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
        self.validate_world(world);
        let change_tick = world.read_change_tick();
        // SAFETY: Param is read-only and doesn't allow mutable access to World. It also matches the World this SystemState was created with.
        unsafe { self.fetch(world, change_tick) }
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
        self.validate_world(world);
        let change_tick = world.change_tick();
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.fetch(world, change_tick) }
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
        world: &'w World,
    ) -> SystemParamItem<'w, 's, Param> {
        let change_tick = world.increment_change_tick();
        self.fetch(world, change_tick)
    }

    /// # Safety
    /// This call might access any of the input parameters in a way that violates Rust's mutability rules. Make sure the data
    /// access is safe in the context of global [`World`] access. The passed-in [`World`] _must_ be the [`World`] the [`SystemState`] was
    /// created with.
    #[inline]
    unsafe fn fetch<'w, 's>(
        &'s mut self,
        world: &'w World,
        change_tick: Tick,
    ) -> SystemParamItem<'w, 's, Param> {
        let param = Param::get_param(
            &mut self.param_state,
            &self.meta,
            world.as_unsafe_world_cell_migration_internal(),
            change_tick,
        );
        self.meta.last_run = change_tick;
        param
    }
}

impl<Param: SystemParam> FromWorld for SystemState<Param> {
    fn from_world(world: &mut World) -> Self {
        Self::new(world)
    }
}

/// Conversion trait to turn something into a [`System`].
///
/// Use this to get a system from a function. Also note that every system implements this trait as
/// well.
///
/// # Examples
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn my_system_function(a_usize_local: Local<usize>) {}
///
/// let system = IntoSystem::into_system(my_system_function);
/// ```
// This trait has to be generic because we have potentially overlapping impls, in particular
// because Rust thinks a type could impl multiple different `FnMut` combinations
// even though none can currently
pub trait IntoSystem<In, Out, Marker>: Sized {
    type System: System<In = In, Out = Out>;
    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

// Systems implicitly implement IntoSystem
impl<In, Out, Sys: System<In = In, Out = Out>> IntoSystem<In, Out, ()> for Sys {
    type System = Sys;
    fn into_system(this: Self) -> Sys {
        this
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystems`](FunctionSystem) the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input parameters.
///
/// # Examples
///
/// Here is a simple example of a system that takes a [`usize`] returning the square of it.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut square_system = IntoSystem::into_system(square);
///
///     let mut world = World::default();
///     square_system.initialize(&mut world);
///     assert_eq!(square_system.run(12, &mut world), 144);
/// }
///
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
/// ```
pub struct In<In>(pub In);

/// The [`System`] counter part of an ordinary function.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`SystemParam`]s. The output of the system becomes the functions return type, while the input
/// becomes the functions [`In`] tagged parameter or `()` if no such parameter exists.
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
    param_state: Option<<F::Param as SystemParam>::State>,
    system_meta: SystemMeta,
    world_id: Option<WorldId>,
    archetype_generation: ArchetypeGeneration,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> Marker>,
}

// De-initializes the cloned system.
impl<Marker, F> Clone for FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            world_id: None,
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

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
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            world_id: None,
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

impl<Marker, F> FunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    /// Message shown when a system isn't initialised
    // When lines get too long, rustfmt can sometimes refuse to format them.
    // Work around this by storing the message separately.
    const PARAM_MESSAGE: &'static str = "System's param_state was not found. Did you forget to initialize this system before running it?";
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
    fn type_id(&self) -> TypeId {
        TypeId::of::<F>()
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
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        let change_tick = world.increment_change_tick();

        // Safety:
        // We update the archetype component access correctly based on `Param`'s requirements
        // in `update_archetype_component_access`.
        // Our caller upholds the requirements.
        let params = F::Param::get_param(
            self.param_state.as_mut().expect(Self::PARAM_MESSAGE),
            &self.system_meta,
            world.as_unsafe_world_cell_migration_internal(),
            change_tick,
        );
        let out = self.func.run(input, params);
        self.system_meta.last_run = change_tick;
        out
    }

    fn get_last_run(&self) -> Tick {
        self.system_meta.last_run
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system_meta.last_run = last_run;
    }

    #[inline]
    fn apply_buffers(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().expect(Self::PARAM_MESSAGE);
        F::Param::apply(param_state, &self.system_meta, world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.world_id = Some(world.id());
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
        self.param_state = Some(F::Param::init_state(world, &mut self.system_meta));
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        assert!(self.world_id == Some(world.id()), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            let param_state = self.param_state.as_mut().unwrap();
            F::Param::new_archetype(
                param_state,
                &archetypes[ArchetypeId::new(archetype_index)],
                &mut self.system_meta,
            );
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

    fn default_system_sets(&self) -> Vec<Box<dyn crate::schedule::SystemSet>> {
        let set = crate::schedule::SystemTypeSet::<F>::new();
        vec![Box::new(set)]
    }
}

/// SAFETY: `F`'s param is `ReadOnlySystemParam`, so this system will only read from the world.
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
/// ```rust
/// use std::num::ParseIntError;
///
/// use bevy_ecs::prelude::*;
///
/// /// Pipe creates a new system which calls `a`, then calls `b` with the output of `a`
/// pub fn pipe<A, B, AMarker, BMarker>(
///     mut a: A,
///     mut b: B,
/// ) -> impl FnMut(In<A::In>, ParamSet<(A::Param, B::Param)>) -> B::Out
/// where
///     // We need A and B to be systems, add those bounds
///     A: SystemParamFunction<AMarker>,
///     B: SystemParamFunction<BMarker, In = A::Out>,
/// {
///     // The type of `params` is inferred based on the return of this function above
///     move |In(a_in), mut params| {
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
pub trait SystemParamFunction<Marker>: Send + Sync + 'static {
    /// The input type to this system. See [`System::In`].
    type In;

    /// The return type of this system. See [`System::Out`].
    type Out;

    /// The [`SystemParam`]/s used by this system to access the [`World`].
    type Param: SystemParam;

    /// Executes this system once. See [`System::run`] or [`System::run_unsafe`].
    fn run(&mut self, input: Self::In, param_value: SystemParamItem<Self::Param>) -> Self::Out;
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<fn($($param,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut($($param),*) -> Out +
                FnMut($(SystemParamItem<$param>),*) -> Out, Out: 'static
        {
            type In = ();
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, _input: (), param_value: SystemParamItem< ($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognise that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
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

        #[allow(non_snake_case)]
        impl<Input, Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<fn(In<Input>, $($param,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut(In<Input>, $($param),*) -> Out +
                FnMut(In<Input>, $(SystemParamItem<$param>),*) -> Out, Out: 'static
        {
            type In = Input;
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, input: Input, param_value: SystemParamItem< ($($param,)*)>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($param,)*>(
                    mut f: impl FnMut(In<Input>, $($param,)*)->Out,
                    input: In<Input>,
                    $($param: $param,)*
                )->Out{
                    f(input, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, In(input), $($param),*)
            }
        }
    };
}

// Note that we rely on the highest impl to be <= the highest order of the tuple impls
// of `SystemParam` created.
all_tuples!(impl_system_function, 0, 16, F);
