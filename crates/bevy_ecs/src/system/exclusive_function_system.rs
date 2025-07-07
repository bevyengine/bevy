use crate::{
    component::{CheckChangeTicks, ComponentId, Tick},
    error::Result,
    query::FilteredAccessSet,
    schedule::{InternedSystemSet, SystemSet},
    system::{
        check_system_change_tick, ExclusiveSystemParam, ExclusiveSystemParamItem, IntoResult,
        IntoSystem, System, SystemIn, SystemInput, SystemMeta,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use alloc::{borrow::Cow, vec, vec::Vec};
use bevy_utils::prelude::DebugName;
use core::marker::PhantomData;
use variadics_please::all_tuples;

use super::{RunSystemError, SystemParamValidationError, SystemStateFlags};

/// A function system that runs with exclusive [`World`] access.
///
/// You get this by calling [`IntoSystem::into_system`]  on a function that only accepts
/// [`ExclusiveSystemParam`]s.
///
/// [`ExclusiveFunctionSystem`] must be `.initialized` before they can be run.
pub struct ExclusiveFunctionSystem<Marker, Out, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    func: F,
    #[cfg(feature = "hotpatching")]
    current_ptr: subsecond::HotFnPtr,
    param_state: Option<<F::Param as ExclusiveSystemParam>::State>,
    system_meta: SystemMeta,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> (Marker, Out)>,
}

impl<Marker, Out, F> ExclusiveFunctionSystem<Marker, Out, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    /// Return this system with a new name.
    ///
    /// Useful to give closure systems more readable and unique names for debugging and tracing.
    pub fn with_name(mut self, new_name: impl Into<Cow<'static, str>>) -> Self {
        self.system_meta.set_name(new_name);
        self
    }
}

/// A marker type used to distinguish exclusive function systems from regular function systems.
#[doc(hidden)]
pub struct IsExclusiveFunctionSystem;

impl<Out, Marker, F> IntoSystem<F::In, Out, (IsExclusiveFunctionSystem, Marker, Out)> for F
where
    Out: 'static,
    Marker: 'static,
    F::Out: IntoResult<Out>,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type System = ExclusiveFunctionSystem<Marker, Out, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusiveFunctionSystem {
            func,
            #[cfg(feature = "hotpatching")]
            current_ptr: subsecond::HotFn::current(
                <F as ExclusiveSystemParamFunction<Marker>>::run,
            )
            .ptr_address(),
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}

const PARAM_MESSAGE: &str = "System's param_state was not found. Did you forget to initialize this system before running it?";

impl<Marker, Out, F> System for ExclusiveFunctionSystem<Marker, Out, F>
where
    Marker: 'static,
    Out: 'static,
    F::Out: IntoResult<Out>,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type In = F::In;
    type Out = Out;

    #[inline]
    fn name(&self) -> DebugName {
        self.system_meta.name.clone()
    }

    #[inline]
    fn flags(&self) -> SystemStateFlags {
        // non-send , exclusive , no deferred
        // the executor runs exclusive systems on the main thread, so this
        // field reflects that constraint
        // exclusive systems have no deferred system params
        SystemStateFlags::NON_SEND | SystemStateFlags::EXCLUSIVE
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Result<Self::Out, RunSystemError> {
        // SAFETY: The safety is upheld by the caller.
        let world = unsafe { world.world_mut() };
        world.last_change_tick_scope(self.system_meta.last_run, |world| {
            #[cfg(feature = "trace")]
            let _span_guard = self.system_meta.system_span.enter();

            let params = F::Param::get_param(
                self.param_state.as_mut().expect(PARAM_MESSAGE),
                &self.system_meta,
            );

            #[cfg(feature = "hotpatching")]
            let out = {
                let mut hot_fn =
                    subsecond::HotFn::current(<F as ExclusiveSystemParamFunction<Marker>>::run);
                // SAFETY:
                // - pointer used to call is from the current jump table
                unsafe {
                    hot_fn
                        .try_call_with_ptr(self.current_ptr, (&mut self.func, world, input, params))
                        .expect("Error calling hotpatched system. Run a full rebuild")
                }
            };
            #[cfg(not(feature = "hotpatching"))]
            let out = self.func.run(world, input, params);

            world.flush();
            self.system_meta.last_run = world.increment_change_tick();

            IntoResult::into_result(out)
        })
    }

    #[cfg(feature = "hotpatching")]
    #[inline]
    fn refresh_hotpatch(&mut self) {
        let new = subsecond::HotFn::current(<F as ExclusiveSystemParamFunction<Marker>>::run)
            .ptr_address();
        if new != self.current_ptr {
            log::debug!("system {} hotpatched", self.name());
        }
        self.current_ptr = new;
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
    unsafe fn validate_param_unsafe(
        &mut self,
        _world: UnsafeWorldCell,
    ) -> Result<(), SystemParamValidationError> {
        // All exclusive system params are always available.
        Ok(())
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) -> FilteredAccessSet<ComponentId> {
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
        self.param_state = Some(F::Param::init(world, &mut self.system_meta));
        FilteredAccessSet::new()
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
    type In: SystemInput;

    /// The return type of this system. See [`System::Out`].
    type Out;

    /// The [`ExclusiveSystemParam`]'s defined by this system's `fn` parameters.
    type Param: ExclusiveSystemParam;

    /// Executes this system once. See [`System::run`].
    fn run(
        &mut self,
        world: &mut World,
        input: <Self::In as SystemInput>::Inner<'_>,
        param_value: ExclusiveSystemParamItem<Self::Param>,
    ) -> Self::Out;
}

/// A marker type used to distinguish exclusive function systems with and without input.
#[doc(hidden)]
pub struct HasExclusiveSystemInput;

macro_rules! impl_exclusive_system_function {
    ($($param: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is within a macro, and as such, the below lints may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "Certain variable names are provided by the caller, not by us."
        )]
        impl<Out, Func, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<fn($($param,)*) -> Out> for Func
        where
            Func: Send + Sync + 'static,
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

        #[expect(
            clippy::allow_attributes,
            reason = "This is within a macro, and as such, the below lints may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "Certain variable names are provided by the caller, not by us."
        )]
        impl<In, Out, Func, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<(HasExclusiveSystemInput, fn(In, $($param,)*) -> Out)> for Func
        where
            Func: Send + Sync + 'static,
            for <'a> &'a mut Func:
                FnMut(In, &mut World, $($param),*) -> Out +
                FnMut(In::Param<'_>, &mut World, $(ExclusiveSystemParamItem<$param>),*) -> Out,
            In: SystemInput + 'static,
            Out: 'static,
        {
            type In = In;
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, world: &mut World, input: In::Inner<'_>, param_value: ExclusiveSystemParamItem< ($($param,)*)>) -> Out {
                // Yes, this is strange, but `rustc` fails to compile this impl
                // without using this function. It fails to recognize that `func`
                // is a function, potentially because of the multiple impls of `FnMut`
                fn call_inner<In: SystemInput, Out, $($param,)*>(
                    _: PhantomData<In>,
                    mut f: impl FnMut(In::Param<'_>, &mut World, $($param,)*) -> Out,
                    input: In::Inner<'_>,
                    world: &mut World,
                    $($param: $param,)*
                ) -> Out {
                    f(In::wrap(input), world, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(PhantomData::<In>, self, input, world, $($param),*)
            }
        }
    };
}
// Note that we rely on the highest impl to be <= the highest order of the tuple impls
// of `SystemParam` created.
all_tuples!(impl_exclusive_system_function, 0, 16, F);

#[cfg(test)]
mod tests {
    use crate::system::input::SystemInput;

    use super::*;

    #[test]
    fn into_system_type_id_consistency() {
        fn test<T, In: SystemInput, Out, Marker>(function: T)
        where
            T: IntoSystem<In, Out, Marker> + Copy,
        {
            fn reference_system(_world: &mut World) {}

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

        fn exclusive_function_system(_world: &mut World) {}

        test(exclusive_function_system);
    }
}
