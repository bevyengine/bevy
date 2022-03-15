use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::ComponentId,
    query::{Access, FilteredAccessSet},
    system::{
        check_system_change_tick, ReadOnlySystemParamFetch, System, SystemParam, SystemParamFetch,
        SystemParamItem, SystemParamState,
    },
    world::{World, WorldId},
};
use bevy_ecs_macros::all_tuples;
use std::{borrow::Cow, marker::PhantomData};

/// The metadata of a [`System`].
pub struct SystemMeta {
    pub(crate) name: Cow<'static, str>,
    pub(crate) component_access_set: FilteredAccessSet<ComponentId>,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    // NOTE: this must be kept private. making a SystemMeta non-send is irreversible to prevent
    // SystemParams from overriding each other
    is_send: bool,
    pub(crate) last_change_tick: u32,
}

impl SystemMeta {
    fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            archetype_component_access: Access::default(),
            component_access_set: FilteredAccessSet::default(),
            is_send: true,
            last_change_tick: 0,
        }
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

    #[inline]
    pub(crate) fn check_change_tick(&mut self, change_tick: u32) {
        check_system_change_tick(&mut self.last_change_tick, change_tick, self.name.as_ref());
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
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::{system::SystemState};
/// use bevy_ecs::event::Events;
///
/// struct MyEvent;
/// struct MyResource(u32);
///
/// #[derive(Component)]
/// struct MyComponent;
///
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
///     )> = SystemState::new(&mut world);
///
/// // Use system_state.get_mut(&mut world) and unpack your system parameters into variables!
/// // system_state.get(&world) provides read-only versions of your system parameters instead.
/// let (event_writer, maybe_resource, query) = system_state.get_mut(&mut world);
/// ```
/// Caching:
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::{system::SystemState};
/// use bevy_ecs::event::Events;
///
/// struct MyEvent;
/// struct CachedSystemState<'w, 's>{
///    event_state: SystemState<EventReader<'w, 's, MyEvent>>
/// }
///
/// // Create and store a system state once
/// let mut world = World::new();
/// world.init_resource::<Events<MyEvent>>();
/// let initial_state: SystemState<EventReader<MyEvent>>  = SystemState::new(&mut world);
///
/// // The system state is cached in a resource
/// world.insert_resource(CachedSystemState{event_state: initial_state});
///
/// // Later, fetch the cached system state, saving on overhead
/// world.resource_scope(|world, mut cached_state: Mut<CachedSystemState>| {
///     let mut event_reader = cached_state.event_state.get_mut(world);
///
///     for events in event_reader.iter(){
///         println!("Hello World!");
///     };
/// });
/// ```
pub struct SystemState<Param: SystemParam> {
    meta: SystemMeta,
    param_state: <Param as SystemParam>::Fetch,
    world_id: WorldId,
    archetype_generation: ArchetypeGeneration,
}

impl<Param: SystemParam> SystemState<Param> {
    pub fn new(world: &mut World) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        let param_state = <Param::Fetch as SystemParamState>::init(world, &mut meta);
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
    pub fn get<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item
    where
        Param::Fetch: ReadOnlySystemParamFetch,
    {
        self.validate_world_and_update_archetypes(world);
        // SAFE: Param is read-only and doesn't allow mutable access to World. It also matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Retrieve the mutable [`SystemParam`] values.
    #[inline]
    pub fn get_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        self.validate_world_and_update_archetypes(world);
        // SAFE: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Applies all state queued up for [`SystemParam`] values. For example, this will apply commands queued up
    /// by a [`Commands`](`super::Commands`) parameter to the given [`World`].
    /// This function should be called manually after the values returned by [`SystemState::get`] and [`SystemState::get_mut`]
    /// are finished being used.
    pub fn apply(&mut self, world: &mut World) {
        self.param_state.apply(world);
    }

    #[inline]
    pub fn matches_world(&self, world: &World) -> bool {
        self.world_id == world.id()
    }

    pub(crate) fn new_archetype(&mut self, archetype: &Archetype) {
        self.param_state.new_archetype(archetype, &mut self.meta);
    }

    fn validate_world_and_update_archetypes(&mut self, world: &World) {
        assert!(self.matches_world(world), "Encountered a mismatched World. A SystemState cannot be used with Worlds other than the one it was created with.");
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            self.param_state.new_archetype(
                &archetypes[ArchetypeId::new(archetype_index)],
                &mut self.meta,
            );
        }
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
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        let change_tick = world.increment_change_tick();
        let param = <Param::Fetch as SystemParamFetch>::get_param(
            &mut self.param_state,
            &self.meta,
            world,
            change_tick,
        );
        self.meta.last_change_tick = change_tick;
        param
    }
}

/// A trait for defining systems with a [`SystemParam`] associated type.
///
/// This facilitates the creation of systems that are generic over some trait
/// and that use that trait's associated types as `SystemParam`s.
pub trait RunSystem: Send + Sync + 'static {
    /// The `SystemParam` type passed to the system when it runs.
    type Param: SystemParam;

    /// Runs the system.
    fn run(param: SystemParamItem<Self::Param>);

    /// Creates a concrete instance of the system for the specified `World`.
    fn system(world: &mut World) -> ParamSystem<Self::Param> {
        ParamSystem {
            run: Self::run,
            state: SystemState::new(world),
        }
    }
}

pub struct ParamSystem<P: SystemParam> {
    state: SystemState<P>,
    run: fn(SystemParamItem<P>),
}

impl<P: SystemParam + 'static> System for ParamSystem<P> {
    type In = ();

    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.state.meta().name.clone()
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        self.state.new_archetype(archetype);
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.state.meta().component_access_set.combined_access()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.state.meta().archetype_component_access
    }

    fn is_send(&self) -> bool {
        self.state.meta().is_send()
    }

    unsafe fn run_unsafe(&mut self, _input: Self::In, world: &World) -> Self::Out {
        let param = self.state.get_unchecked_manual(world);
        (self.run)(param);
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.state.apply(world);
    }

    fn initialize(&mut self, _world: &mut World) {
        // already initialized by nature of the SystemState being constructed
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.state.meta.check_change_tick(change_tick);
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
/// use bevy_ecs::system::IntoSystem;
/// use bevy_ecs::system::Res;
///
/// fn my_system_function(an_usize_resource: Res<usize>) {}
///
/// let system = IntoSystem::system(my_system_function);
/// ```
// This trait has to be generic because we have potentially overlapping impls, in particular
// because Rust thinks a type could impl multiple different `FnMut` combinations
// even though none can currently
pub trait IntoSystem<In, Out, Params>: Sized {
    type System: System<In = In, Out = Out>;
    /// Turns this value into its corresponding [`System`].
    ///
    /// Use of this method was formerly required whenever adding a `system` to an `App`.
    /// or other cases where a system is required.
    /// However, since [#2398](https://github.com/bevyengine/bevy/pull/2398),
    /// this is no longer required.
    ///
    /// In future, this method will be removed.
    ///
    /// One use of this method is to assert that a given function is a valid system.
    /// For this case, use [`bevy_ecs::system::assert_is_system`] instead.
    ///
    /// [`bevy_ecs::system::assert_is_system`]: [`crate::system::assert_is_system`]:
    #[deprecated(
        since = "0.7.0",
        note = "`.system()` is no longer needed, as methods which accept systems will convert functions into a system automatically"
    )]
    fn system(self) -> Self::System {
        IntoSystem::into_system(self)
    }
    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

pub struct AlreadyWasSystem;

// Systems implicitly implement IntoSystem
impl<In, Out, Sys: System<In = In, Out = Out>> IntoSystem<In, Out, AlreadyWasSystem> for Sys {
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
/// means a system can only have one or zero input paramaters.
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
pub struct InputMarker;

/// The [`System`] counter part of an ordinary function.
///
/// You get this by calling [`IntoSystem::system`]  on a function that only accepts
/// [`SystemParam`]s. The output of the system becomes the functions return type, while the input
/// becomes the functions [`In`] tagged parameter or `()` if no such parameter exists.
pub struct FunctionSystem<In, Out, Param, Marker, F>
where
    Param: SystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    system_meta: SystemMeta,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    #[allow(clippy::type_complexity)]
    marker: PhantomData<fn() -> (In, Out, Marker)>,
}

pub struct IsFunctionSystem;

impl<In, Out, Param, Marker, F> IntoSystem<In, Out, (IsFunctionSystem, Param, Marker)> for F
where
    In: 'static,
    Out: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    type System = FunctionSystem<In, Out, Param, Marker, F>;
    fn into_system(func: Self) -> Self::System {
        FunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}

impl<In, Out, Param, Marker, F> System for FunctionSystem<In, Out, Param, Marker, F>
where
    In: 'static,
    Out: 'static,
    Param: SystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    type In = In;
    type Out = Out;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.system_meta.name.clone()
    }

    #[inline]
    fn new_archetype(&mut self, archetype: &Archetype) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.new_archetype(archetype, &mut self.system_meta);
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
    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        let change_tick = world.increment_change_tick();
        let out = self.func.run(
            input,
            self.param_state.as_mut().unwrap(),
            &self.system_meta,
            world,
            change_tick,
        );
        self.system_meta.last_change_tick = change_tick;
        out
    }

    #[inline]
    fn apply_buffers(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.apply(world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.param_state = Some(<Param::Fetch as SystemParamState>::init(
            world,
            &mut self.system_meta,
        ));
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: u32) {
        check_system_change_tick(
            &mut self.system_meta.last_change_tick,
            change_tick,
            self.system_meta.name.as_ref(),
        );
    }
}

/// A trait implemented for all functions that can be used as [`System`]s.
pub trait SystemParamFunction<In, Out, Param: SystemParam, Marker>: Send + Sync + 'static {
    /// # Safety
    ///
    /// This call might access any of the input parameters in an unsafe way. Make sure the data
    /// access is safe in the context of the system scheduler.
    unsafe fn run(
        &mut self,
        input: In,
        state: &mut Param::Fetch,
        system_meta: &SystemMeta,
        world: &World,
        change_tick: u32,
    ) -> Out;
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<(), Out, ($($param,)*), ()> for Func
        where
        for <'a> &'a mut Func:
                FnMut($($param),*) -> Out +
                FnMut($(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Out, Out: 'static
        {
            #[inline]
            unsafe fn run(&mut self, _input: (), state: &mut <($($param,)*) as SystemParam>::Fetch, system_meta: &SystemMeta, world: &World, change_tick: u32) -> Out {
                // Yes, this is strange, but rustc fails to compile this impl
                // without using this function.
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($param,)*>(
                    mut f: impl FnMut($($param,)*)->Out,
                    $($param: $param,)*
                )->Out{
                    f($($param,)*)
                }
                let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::get_param(state, system_meta, world, change_tick);
                call_inner(self, $($param),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<Input, Out, ($($param,)*), InputMarker> for Func
        where
        for <'a> &'a mut Func:
                FnMut(In<Input>, $($param),*) -> Out +
                FnMut(In<Input>, $(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Out, Out: 'static
        {
            #[inline]
            unsafe fn run(&mut self, input: Input, state: &mut <($($param,)*) as SystemParam>::Fetch, system_meta: &SystemMeta, world: &World, change_tick: u32) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($param,)*>(
                    mut f: impl FnMut(In<Input>, $($param,)*)->Out,
                    input: In<Input>,
                    $($param: $param,)*
                )->Out{
                    f(input, $($param,)*)
                }
                let ($($param,)*) = <<($($param,)*) as SystemParam>::Fetch as SystemParamFetch>::get_param(state, system_meta, world, change_tick);
                call_inner(self, In(input), $($param),*)
            }
        }
    };
}

all_tuples!(impl_system_function, 0, 16, F);
