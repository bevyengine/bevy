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
#[derive(Debug, Clone)]
pub struct SystemMeta {
    pub(crate) name: Cow<'static, str>,
    pub(crate) component_access_set: FilteredAccessSet<ComponentId>,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    pub(crate) last_change_tick: u32,
    // NOTE: this must be kept private. making a SystemMeta non-send is irreversible to prevent
    // SystemParams from overriding each other
    is_send: bool,
}

impl SystemMeta {
    fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            archetype_component_access: Access::default(),
            component_access_set: FilteredAccessSet::default(),
            last_change_tick: 0,
            is_send: true,
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
/// `SystemState` can make working with `&mut World` more convenient, especially when used in combination
/// with [`World::resource_scope`].
///
/// # Notes
///
/// [`SystemState`] instances can be cached to improve performance,
/// and *must* be cached and reused in order for params that rely on local state to work correctly.
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
/// // Use system_state.get_mut(&mut world) and unpack your system parameters into variables!///
/// // You can use system_state.get(&world) if all your parameters are read-only or local.
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

    /// Retrieves the [`SystemParam`] values (must all be read-only) from the [`World`].
    ///
    /// This method also ensures the state's [`Access`] is up-to-date before retrieving the data.
    #[inline]
    pub fn get<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item
    where
        Param::Fetch: ReadOnlySystemParamFetch,
    {
        self.validate_world_and_update_archetypes(world);
        // SAFETY: The params cannot request mutable access and world is the same one used to construct this state.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Retrieves the [`SystemParam`] values from the [`World`].
    ///
    /// This method also ensures the state's [`Access`] is up-to-date before retrieving the data.
    #[inline]
    pub fn get_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        self.validate_world_and_update_archetypes(world);
        // SAFETY: World is uniquely borrowed and matches the World this SystemState was created with.
        unsafe { self.get_unchecked_manual(world) }
    }

    /// Applies all state queued by the [`SystemParam`] values onto the [`World`].
    /// For example, this will apply any [commands](crate::system::Command) queued with
    /// [`Commands`](crate::system::Commands).
    ///
    /// Call once the data borrowed by [`SystemState::get`] and [`SystemState::get_mut`] is done being used.
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
    /// Retrieves the [`SystemParam`] values from the [`World`].
    ///
    /// This method does _not_ update the state's [`Access`] before retrieving the data.
    ///
    /// # Safety    
    ///
    /// Caller must ensure:
    /// - The given world is the same world used to construct the system state.
    /// - There are no active references that conflict with the system state's access. Mutable access must be unique.
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

/// Conversion trait to turn something into a [`System`]. Use this to get a system from a function or closure.
///
/// This trait is blanket implemented for all [`System`] types.
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
// This trait requires the generic `Params` because, as far as Rust knows, a type could have
// several impls of `FnMut` with different arguments, even though functions and closures don't.
pub trait IntoSystem<In, Out, Params>: Sized {
    type System: System<In = In, Out = Out>;
    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

pub struct AlreadyWasSystem;

// Converting a system into a system is a no-op.
impl<In, Out, Sys: System<In = In, Out = Out>> IntoSystem<In, Out, AlreadyWasSystem> for Sys {
    type System = Sys;
    fn into_system(this: Self) -> Sys {
        this
    }
}

/// A system parameter that denotes an external input.
///
/// The input for a [`System`] object must be passed into [`run`](System::run).
///
/// To use an `In<T>` with a [`FunctionSystem`](FunctionSystem), it has to be first parameter
/// in its function signature.
///
/// # Examples
///
/// This system takes an external [`usize`] and returns its square.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
///
/// fn main() {
///     let mut square_system = IntoSystem::into_system(square);
///
///     let mut world = World::default();
///     square_system.initialize(&mut world);
///     assert_eq!(square_system.run(12, &mut world), 144);
/// }
/// ```
pub struct In<T>(pub T);
#[doc(hidden)]
pub struct InputMarker;

/// The [`System`]-type of functions and closures.
///
/// Constructed by calling [`IntoSystem::into_system`] with a function or closure whose arguments all implement
/// [`SystemParam`].
///
/// If the function's first argument is [`In<T>`], `T` becomes the system's [`In`](crate::system::System::In) type
/// (`In = ()` otherwise).
/// The function's return type becomes the system's [`Out`](crate::system::System::Out) type.
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

/// Implicit conversion of [`System`] types into a [`SystemLabel`] (or several).
///
/// For example, `System`-compatible functions are converted into a [`SystemTypeIdLabel`].
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
