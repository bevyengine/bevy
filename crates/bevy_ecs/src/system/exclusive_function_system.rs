use crate::{
    archetype::ArchetypeComponentId,
    change_detection::MAX_CHANGE_AGE,
    component::ComponentId,
    query::Access,
    schedule::{SystemLabel, SystemLabelId},
    system::{
        check_system_change_tick, AsSystemLabel, ExclusiveSystemParam, ExclusiveSystemParamFetch,
        ExclusiveSystemParamItem, ExclusiveSystemParamState, IntoSystem, System, SystemMeta,
        SystemTypeIdLabel,
    },
    world::{World, WorldId},
};
use bevy_ecs_macros::all_tuples;
use std::{borrow::Cow, marker::PhantomData};

/// A function system that runs with exclusive [`World`] access.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`ExclusiveSystemParam`]s.
///
/// [`ExclusiveFunctionSystem`] must be `.initialized` before they can be run.
pub struct ExclusiveFunctionSystem<Param, Marker, F>
where
    Param: ExclusiveSystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    system_meta: SystemMeta,
    world_id: Option<WorldId>,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> Marker>,
}

pub struct IsExclusiveFunctionSystem;

impl<Param, Marker, F> IntoSystem<(), (), (IsExclusiveFunctionSystem, Param, Marker)> for F
where
    Param: ExclusiveSystemParam + 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Param, Marker> + Send + Sync + 'static,
{
    type System = ExclusiveFunctionSystem<Param, Marker, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusiveFunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            world_id: None,
            marker: PhantomData,
        }
    }
}

const PARAM_MESSAGE: &str = "System's param_state was not found. Did you forget to initialize this system before running it?";

impl<Param, Marker, F> System for ExclusiveFunctionSystem<Param, Marker, F>
where
    Param: ExclusiveSystemParam + 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Param, Marker> + Send + Sync + 'static,
{
    type In = ();
    type Out = ();

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
        // exclusive systems should have access to non-send resources
        // the executor runs exclusive systems on the main thread, so this
        // field reflects that constraint
        false
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: &World) -> Self::Out {
        panic!("Cannot run exclusive systems with a shared World reference");
    }

    fn run(&mut self, _input: Self::In, world: &mut World) -> Self::Out {
        let saved_last_tick = world.last_change_tick;
        world.last_change_tick = self.system_meta.last_change_tick;

        let params = <Param as ExclusiveSystemParam>::Fetch::get_param(
            self.param_state.as_mut().expect(PARAM_MESSAGE),
            &self.system_meta,
        );
        self.func.run(world, params);

        let change_tick = world.change_tick.get_mut();
        self.system_meta.last_change_tick = *change_tick;
        *change_tick += 1;
        world.last_change_tick = saved_last_tick;
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
        let param_state = self.param_state.as_mut().expect(PARAM_MESSAGE);
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

    fn update_archetype_component_access(&mut self, _world: &World) {}

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

impl<Param: ExclusiveSystemParam, Marker, T: ExclusiveSystemParamFunction<Param, Marker>>
    AsSystemLabel<(Param, Marker, IsExclusiveFunctionSystem)> for T
{
    #[inline]
    fn as_system_label(&self) -> SystemLabelId {
        SystemTypeIdLabel::<T>(PhantomData).as_label()
    }
}

/// A trait implemented for all exclusive system functions that can be used as [`System`]s.
///
/// This trait can be useful for making your own systems which accept other systems,
/// sometimes called higher order systems.
pub trait ExclusiveSystemParamFunction<Param: ExclusiveSystemParam, Marker>:
    Send + Sync + 'static
{
    fn run(&mut self, world: &mut World, param_value: ExclusiveSystemParamItem<Param>);
}

macro_rules! impl_exclusive_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Func: Send + Sync + 'static, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<($($param,)*), ()> for Func
        where
        for <'a> &'a mut Func:
                FnMut(&mut World, $($param),*) +
                FnMut(&mut World, $(ExclusiveSystemParamItem<$param>),*)
        {
            #[inline]
            fn run(&mut self, world: &mut World, param_value: ExclusiveSystemParamItem< ($($param,)*)>) {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognise that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
                fn call_inner<$($param,)*>(
                    mut f: impl FnMut(&mut World, $($param,)*),
                    world: &mut World,
                    $($param: $param,)*
                ) {
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
