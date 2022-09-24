use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    change_detection::MAX_CHANGE_AGE,
    component::ComponentId,
    prelude::QueryState,
    query::{Access, ReadOnlyWorldQuery, WorldQuery},
    schedule::{SystemLabel, SystemLabelId},
    system::{
        check_system_change_tick, AsSystemLabel, IntoSystem, System, SystemMeta, SystemParam,
        SystemState, SystemTypeIdLabel,
    },
    world::{World, WorldId},
};
use bevy_ecs_macros::all_tuples;
use std::{borrow::Cow, marker::PhantomData};

/// The [`System`] counter part of an ordinary function.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`SystemParam`]s. The output of the system becomes the functions return type, while the input
/// becomes the functions [`In`] tagged parameter or `()` if no such parameter exists.
///
/// [`FunctionSystem`] must be `.initialized` before they can be run.
pub struct ExclusiveFunctionSystem<In, Out, Param, Marker, F>
where
    Param: ExclusiveSystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    system_meta: SystemMeta,
    world_id: Option<WorldId>,
    archetype_generation: ArchetypeGeneration,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> (In, Out, Marker)>,
}

pub struct IsExclusiveFunctionSystem;

impl<In, Out, Param, Marker, F> IntoSystem<In, Out, (IsExclusiveFunctionSystem, Param, Marker)>
    for F
where
    In: 'static,
    Out: 'static,
    Param: ExclusiveSystemParam + 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
{
    type System = ExclusiveFunctionSystem<In, Out, Param, Marker, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusiveFunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            world_id: None,
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}

impl<In, Out, Param, Marker, F> ExclusiveFunctionSystem<In, Out, Param, Marker, F>
where
    Param: ExclusiveSystemParam,
{
    /// Message shown when a system isn't initialised
    // When lines get too long, rustfmt can sometimes refuse to format them.
    // Work around this by storing the message separately.
    const PARAM_MESSAGE: &'static str = "System's param_state was not found. Did you forget to initialize this system before running it?";
}

impl<In, Out, Param, Marker, F> System for ExclusiveFunctionSystem<In, Out, Param, Marker, F>
where
    In: 'static,
    Out: 'static,
    Param: ExclusiveSystemParam + 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<In, Out, Param, Marker> + Send + Sync + 'static,
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
        self.system_meta.is_send()
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: &World) -> Self::Out {
        panic!("Cannot run exclusive systems with a shared World reference");
    }

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        let change_tick = world.increment_change_tick();

        let params = <Param as ExclusiveSystemParam>::Fetch::get_param(
            self.param_state.as_mut().expect(Self::PARAM_MESSAGE),
            &self.system_meta,
            change_tick,
        );
        let out = self.func.run(input, world, params);
        self.system_meta.last_change_tick = change_tick;
        out
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        true
    }

    fn get_last_change_tick(&self) -> u32 {
        self.system_meta.last_change_tick
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.system_meta.last_change_tick = last_change_tick;
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
        self.param_state = Some(<Param::Fetch as ExclusiveSystemParamState>::init(
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

impl<
        In,
        Out,
        Param: ExclusiveSystemParam,
        Marker,
        T: ExclusiveSystemParamFunction<In, Out, Param, Marker>,
    > AsSystemLabel<(In, Out, Param, Marker, IsExclusiveFunctionSystem)> for T
{
    #[inline]
    fn as_system_label(&self) -> SystemLabelId {
        SystemTypeIdLabel::<T>(PhantomData).as_label()
    }
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
/// [`ChainSystem`]: crate::system::ChainSystem
/// [`ParamSet`]: crate::system::ParamSet
pub trait ExclusiveSystemParamFunction<In, Out, Param: ExclusiveSystemParam, Marker>:
    Send + Sync + 'static
{
    fn run(
        &mut self,
        input: In,
        world: &mut World,
        param_value: ExclusiveSystemParamItem<Param>,
    ) -> Out;
}

macro_rules! impl_exclusive_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func: Send + Sync + 'static, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<(), Out, ($($param,)*), ()> for Func
        where
        for <'a> &'a mut Func:
                FnMut(&mut World, $($param),*) -> Out +
                FnMut(&mut World, $(ExclusiveSystemParamItem<$param>),*) -> Out, Out: 'static
        {
            #[inline]
            fn run(&mut self, _input: (), world: &mut World, param_value: ExclusiveSystemParamItem< ($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognise that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($param,)*>(
                    mut f: impl FnMut(&mut World, $($param,)*)->Out,
                    world: &mut World,
                    $($param: $param,)*
                )->Out{
                    f(world, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, world, $($param),*)
            }
        }
    };
}
// Note that we rely on the highest impl to be <= the highest order of the tuple impls
// of `SystemParam` created.
all_tuples!(impl_exclusive_system_function, 0, 16, F);

pub trait ExclusiveSystemParam: Sized {
    type Fetch: for<'s> ExclusiveSystemParamFetch<'s>;
}

pub type ExclusiveSystemParamItem<'s, P> =
    <<P as ExclusiveSystemParam>::Fetch as ExclusiveSystemParamFetch<'s>>::Item;

/// The state of a [`SystemParam`].
pub trait ExclusiveSystemParamState: Send + Sync + 'static {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self;
    #[inline]
    fn new_archetype(&mut self, _archetype: &Archetype, _system_meta: &mut SystemMeta) {}
    #[inline]
    fn apply(&mut self, _world: &mut World) {}
}

pub trait ExclusiveSystemParamFetch<'state>: ExclusiveSystemParamState {
    type Item: ExclusiveSystemParam<Fetch = Self>;
    fn get_param(state: &'state mut Self, system_meta: &SystemMeta, change_tick: u32)
        -> Self::Item;
}

impl<'a, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParam
    for &'a mut QueryState<Q, F>
{
    type Fetch = QueryState<Q, F>;
}

impl<'s, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParamFetch<'s>
    for QueryState<Q, F>
{
    type Item = &'s mut QueryState<Q, F>;

    fn get_param(state: &'s mut Self, _system_meta: &SystemMeta, _change_tick: u32) -> Self::Item {
        state
    }
}

impl<Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParamState
    for QueryState<Q, F>
{
    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        QueryState::new(world)
    }
}

impl<'a, P: SystemParam + 'static> ExclusiveSystemParam for &'a mut SystemState<P> {
    type Fetch = SystemState<P>;
}

impl<'s, P: SystemParam + 'static> ExclusiveSystemParamFetch<'s> for SystemState<P> {
    type Item = &'s mut SystemState<P>;

    fn get_param(state: &'s mut Self, _system_meta: &SystemMeta, _change_tick: u32) -> Self::Item {
        state
    }
}

impl<P: SystemParam + 'static> ExclusiveSystemParamState for SystemState<P> {
    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        SystemState::new(world)
    }
}

macro_rules! impl_exclusive_system_param_tuple {
    ($($param: ident),*) => {
        impl<$($param: ExclusiveSystemParam),*> ExclusiveSystemParam for ($($param,)*) {
            type Fetch = ($($param::Fetch,)*);
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'s, $($param: ExclusiveSystemParamFetch<'s>),*> ExclusiveSystemParamFetch<'s> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            #[inline]
            #[allow(clippy::unused_unit)]
            fn get_param(
                state: &'s mut Self,
                system_meta: &SystemMeta,
                change_tick: u32,
            ) -> Self::Item {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_meta, change_tick),)*)
            }
        }

        // SAFETY: implementors of each `SystemParamState` in the tuple have validated their impls
        #[allow(clippy::undocumented_unsafe_blocks)] // false positive by clippy
        #[allow(non_snake_case)]
        impl<$($param: ExclusiveSystemParamState),*> ExclusiveSystemParamState for ($($param,)*) {
            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
                (($($param::init(_world, _system_meta),)*))
            }

            #[inline]
            fn new_archetype(&mut self, _archetype: &Archetype, _system_meta: &mut SystemMeta) {
                let ($($param,)*) = self;
                $($param.new_archetype(_archetype, _system_meta);)*
            }

            #[inline]
            fn apply(&mut self, _world: &mut World) {
                let ($($param,)*) = self;
                $($param.apply(_world);)*
            }
        }
    };
}

all_tuples!(impl_exclusive_system_param_tuple, 0, 16, P);
