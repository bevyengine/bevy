use crate::{
    archetype::ArchetypeComponentId,
    component::{ComponentId, Tick},
    query::Access,
    schedule::{InternedSystemSet, SystemSet},
    system::{
        check_system_change_tick, ExclusiveSystemParam, ExclusiveSystemParamItem, In, IntoSystem,
        System, SystemMeta,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use bevy_utils::all_tuples;
use std::{borrow::Cow, marker::PhantomData};

/// A function system that runs with exclusive [`World`] access.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`ExclusiveSystemParam`]s.
///
/// [`ExclusiveFunctionSystem`] must be `.initialized` before they can be run.
pub struct ExclusiveFunctionSystem<Marker, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    func: F,
    param_state: Option<<F::Param as ExclusiveSystemParam>::State>,
    system_meta: SystemMeta,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> Marker>,
}

/// A marker type used to distinguish exclusive function systems from regular function systems.
#[doc(hidden)]
pub struct IsExclusiveFunctionSystem;

impl<Marker, F> IntoSystem<F::In, F::Out, (IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type System = ExclusiveFunctionSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusiveFunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}

const PARAM_MESSAGE: &str = "System's param_state was not found. Did you forget to initialize this system before running it?";

impl<Marker, F> System for ExclusiveFunctionSystem<Marker, F>
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
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
        // exclusive systems should have access to non-send resources
        // the executor runs exclusive systems on the main thread, so this
        // field reflects that constraint
        false
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        true
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        // exclusive systems have no deferred system params
        false
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: UnsafeWorldCell) -> Self::Out {
        panic!("Cannot run exclusive systems with a shared World reference");
    }

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        world.last_change_tick_scope(self.system_meta.last_run, |world| {
            #[cfg(feature = "trace")]
            let _span_guard = self.system_meta.system_span.enter();

            let params = F::Param::get_param(
                self.param_state.as_mut().expect(PARAM_MESSAGE),
                &self.system_meta,
            );
            let out = self.func.run(world, input, params);

            world.flush();
            let change_tick = world.change_tick.get_mut();
            self.system_meta.last_run.set(*change_tick);
            *change_tick = change_tick.wrapping_add(1);

            out
        })
    }

    #[inline]
    fn apply_deferred(&mut self, _world: &mut World) {
        // "pure" exclusive systems do not have any buffers to apply.
        // Systems made by piping a normal system with an exclusive system
        // might have buffers to apply, but this is handled by `PipeSystem`.
    }

    #[inline]
    fn queue_deferred(&mut self, _world: crate::world::DeferredWorld) {
        // "pure" exclusive systems do not have any buffers to apply.
        // Systems made by piping a normal system with an exclusive system
        // might have buffers to apply, but this is handled by `PipeSystem`.
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
        self.param_state = Some(F::Param::init(world, &mut self.system_meta));
    }

    fn update_archetype_component_access(&mut self, _world: UnsafeWorldCell) {}

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

/// A trait implemented for all exclusive system functions that can be used as [`System`]s.
///
/// This trait can be useful for making your own systems which accept other systems,
/// sometimes called higher order systems.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an exclusive system",
    label = "invalid system"
)]
pub trait ExclusiveSystemParamFunction<Marker>: Send + Sync + 'static {
    /// The input type to this system. See [`System::In`].
    type In;

    /// The return type of this system. See [`System::Out`].
    type Out;

    /// The [`ExclusiveSystemParam`]'s defined by this system's `fn` parameters.
    type Param: ExclusiveSystemParam;

    /// Executes this system once. See [`System::run`].
    fn run(
        &mut self,
        world: &mut World,
        input: Self::In,
        param_value: ExclusiveSystemParamItem<Self::Param>,
    ) -> Self::Out;
}

macro_rules! impl_exclusive_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func: Send + Sync + 'static, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<fn($($param,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut(&mut World, $($param),*) -> Out +
                FnMut(&mut World, $(ExclusiveSystemParamItem<$param>),*) -> Out,
            Out: 'static,
        {
            type In = ();
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, world: &mut World, _in: (), param_value: ExclusiveSystemParamItem< ($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognize that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($param,)*>(
                    mut f: impl FnMut(&mut World, $($param,)*) -> Out,
                    world: &mut World,
                    $($param: $param,)*
                ) -> Out {
                    f(world, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, world, $($param),*)
            }
        }
        #[allow(non_snake_case)]
        impl<Input, Out, Func: Send + Sync + 'static, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<fn(In<Input>, $($param,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut(In<Input>, &mut World, $($param),*) -> Out +
                FnMut(In<Input>, &mut World, $(ExclusiveSystemParamItem<$param>),*) -> Out,
            Out: 'static,
        {
            type In = Input;
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, world: &mut World, input: Input, param_value: ExclusiveSystemParamItem< ($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognize that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($param,)*>(
                    mut f: impl FnMut(In<Input>, &mut World, $($param,)*) -> Out,
                    input: Input,
                    world: &mut World,
                    $($param: $param,)*
                ) -> Out {
                    f(In(input), world, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, input, world, $($param),*)
            }
        }
    };
}
// Note that we rely on the highest impl to be <= the highest order of the tuple impls
// of `SystemParam` created.
all_tuples!(impl_exclusive_system_function, 0, 16, F);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_system_type_id_consistency() {
        fn test<T, In, Out, Marker>(function: T)
        where
            T: IntoSystem<In, Out, Marker> + Copy,
        {
            fn reference_system(_world: &mut World) {}

            use std::any::TypeId;

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

        fn exclusive_function_system(_world: &mut World) {}

        test(exclusive_function_system);
    }
}
