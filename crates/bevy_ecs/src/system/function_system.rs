use crate::{
    archetype::{ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    change_detection::MAX_CHANGE_AGE,
    component::ComponentId,
    prelude::FromWorld,
    query::{Access, FilteredAccessSet},
    schedule::{SystemLabel, SystemLabelId},
    system::{
        check_system_change_tick, ReadOnlySystemParamFetch, System, SystemParam, SystemParamFetch,
        SystemParamItem, SystemParamState,
    },
    world::{World, WorldId},
};
use bevy_ecs_macros::all_tuples;
use std::{borrow::Cow, fmt::Debug, marker::PhantomData};

/// The metadata of a [`System`].
#[derive(Clone)]
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
///     for events in event_reader.iter() {
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
        meta.last_change_tick = world.change_tick().wrapping_sub(MAX_CHANGE_AGE);
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
        // SAFETY: Param is read-only and doesn't allow mutable access to World. It also matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Retrieve the mutable [`SystemParam`] values.
    #[inline]
    pub fn get_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        self.validate_world_and_update_archetypes(world);
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
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
/// use bevy_ecs::system::IntoSystem;
/// use bevy_ecs::system::Res;
///
/// fn my_system_function(an_usize_resource: Res<usize>) {}
///
/// let system = IntoSystem::into_system(my_system_function);
/// ```
// This trait has to be generic because we have potentially overlapping impls, in particular
// because Rust thinks a type could impl multiple different `FnMut` combinations
// even though none can currently
pub trait IntoSystem<In, Out, Params>: Sized {
    type System: System<In = In, Out = Out>;
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
#[doc(hidden)]
pub struct InputMarker;

/// The [`System`] counter part of an ordinary function.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`SystemParam`]s. The output of the system becomes the functions return type, while the input
/// becomes the functions [`In`] tagged parameter or `()` if no such parameter exists.
///
/// [`FunctionSystem`] must be `.initialized` before they can be run.
pub struct FunctionSystem<In, Out, Param, Marker, F>
where
    Param: SystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    system_meta: SystemMeta,
    world_id: Option<WorldId>,
    archetype_generation: ArchetypeGeneration,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
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
            world_id: None,
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

impl<In, Out, Param, Marker, F> FunctionSystem<In, Out, Param, Marker, F>
where
    Param: SystemParam,
{
    /// Message shown when a system isn't initialised
    // When lines get too long, rustfmt can sometimes refuse to format them.
    // Work around this by storing the message separately.
    const PARAM_MESSAGE: &'static str = "System's param_state was not found. Did you forget to initialize this system before running it?";
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

        // Safety:
        // We update the archetype component access correctly based on `Param`'s requirements
        // in `update_archetype_component_access`.
        // Our caller upholds the requirements.
        let params = <Param as SystemParam>::Fetch::get_param(
            self.param_state.as_mut().expect(Self::PARAM_MESSAGE),
            &self.system_meta,
            world,
            change_tick,
        );
        let out = self.func.run(input, params);
        self.system_meta.last_change_tick = change_tick;
        out
    }

    #[inline]
    fn apply_buffers(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().expect(Self::PARAM_MESSAGE);
        param_state.apply(world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.world_id = Some(world.id());
        self.system_meta.last_change_tick = world.change_tick().wrapping_sub(MAX_CHANGE_AGE);
        self.param_state = Some(<Param::Fetch as SystemParamState>::init(
            world,
            &mut self.system_meta,
        ));
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        assert!(self.world_id == Some(world.id()), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            self.param_state.as_mut().unwrap().new_archetype(
                &archetypes[ArchetypeId::new(archetype_index)],
                &mut self.system_meta,
            );
        }
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: u32) {
        check_system_change_tick(
            &mut self.system_meta.last_change_tick,
            change_tick,
            self.system_meta.name.as_ref(),
        );
    }
    fn default_labels(&self) -> Vec<SystemLabelId> {
        vec![self.func.as_system_label().as_label()]
    }
}

/// A [`SystemLabel`] that was automatically generated for a system on the basis of its `TypeId`.
pub struct SystemTypeIdLabel<T: 'static>(PhantomData<fn() -> T>);

impl<T: 'static> SystemLabel for SystemTypeIdLabel<T> {
    #[inline]
    fn as_str(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

impl<T> Debug for SystemTypeIdLabel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SystemTypeIdLabel")
            .field(&std::any::type_name::<T>())
            .finish()
    }
}

impl<T> Clone for SystemTypeIdLabel<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for SystemTypeIdLabel<T> {}

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
/// To create something like [`ChainSystem`], but in entirely safe code.
///
/// ```rust
/// use std::num::ParseIntError;
///
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::{SystemParam, SystemParamItem};
///
/// // Unfortunately, we need all of these generics. `A` is the first system, with its
/// // parameters and marker type required for coherence. `B` is the second system, and
/// // the other generics are for the input/output types of `A` and `B`.
/// /// Chain creates a new system which calls `a`, then calls `b` with the output of `a`
/// pub fn chain<AIn, Shared, BOut, A, AParam, AMarker, B, BParam, BMarker>(
///     mut a: A,
///     mut b: B,
/// ) -> impl FnMut(In<AIn>, ParamSet<(SystemParamItem<AParam>, SystemParamItem<BParam>)>) -> BOut
/// where
///     // We need A and B to be systems, add those bounds
///     A: SystemParamFunction<AIn, Shared, AParam, AMarker>,
///     B: SystemParamFunction<Shared, BOut, BParam, BMarker>,
///     AParam: SystemParam,
///     BParam: SystemParam,
/// {
///     // The type of `params` is inferred based on the return of this function above
///     move |In(a_in), mut params| {
///         let shared = a.run(a_in, params.p0());
///         b.run(shared, params.p1())
///     }
/// }
///
/// // Usage example for `chain`:
/// fn main() {
///     let mut world = World::default();
///     world.insert_resource(Message("42".to_string()));
///
///     // chain the `parse_message_system`'s output into the `filter_system`s input
///     let mut chained_system = IntoSystem::into_system(chain(parse_message, filter));
///     chained_system.initialize(&mut world);
///     assert_eq!(chained_system.run((), &mut world), Some(42));
/// }
///
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
/// [`ChainSystem`]: crate::system::ChainSystem
/// [`ParamSet`]: crate::system::ParamSet
pub trait SystemParamFunction<In, Out, Param: SystemParam, Marker>: Send + Sync + 'static {
    fn run(&mut self, input: In, param_value: SystemParamItem<Param>) -> Out;
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<(), Out, ($($param,)*), ()> for Func
        where
        for <'a> &'a mut Func:
                FnMut($($param),*) -> Out +
                FnMut($(SystemParamItem<$param>),*) -> Out, Out: 'static
        {
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
        impl<Input, Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<Input, Out, ($($param,)*), InputMarker> for Func
        where
        for <'a> &'a mut Func:
                FnMut(In<Input>, $($param),*) -> Out +
                FnMut(In<Input>, $(<<$param as SystemParam>::Fetch as SystemParamFetch>::Item),*) -> Out, Out: 'static
        {
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

/// Used to implicitly convert systems to their default labels. For example, it will convert
/// "system functions" to their [`SystemTypeIdLabel`].
pub trait AsSystemLabel<Marker> {
    fn as_system_label(&self) -> SystemLabelId;
}

impl<In, Out, Param: SystemParam, Marker, T: SystemParamFunction<In, Out, Param, Marker>>
    AsSystemLabel<(In, Out, Param, Marker)> for T
{
    #[inline]
    fn as_system_label(&self) -> SystemLabelId {
        SystemTypeIdLabel::<T>(PhantomData).as_label()
    }
}

impl<T: SystemLabel> AsSystemLabel<()> for T {
    #[inline]
    fn as_system_label(&self) -> SystemLabelId {
        self.as_label()
    }
}
