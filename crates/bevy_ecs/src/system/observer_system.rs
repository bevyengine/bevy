use bevy_utils::all_tuples;

use crate::{
    prelude::{Bundle, Trigger},
    system::{System, SystemParam, SystemParamFunction, SystemParamItem},
};

use super::IntoSystem;

/// Implemented for systems that have an [`Observer`] as the first argument.
///
/// [`Observer`]: crate::observer::Observer
pub trait ObserverSystem<E: 'static, B: Bundle, Out = ()>:
    System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static
{
}

impl<
        E: 'static,
        B: Bundle,
        Out,
        T: System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static,
    > ObserverSystem<E, B, Out> for T
{
}

/// Implemented for systems that convert into [`ObserverSystem`].
pub trait IntoObserverSystem<E: 'static, B: Bundle, M, Out = ()>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B, Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

impl<
        S: IntoSystem<Trigger<'static, E, B>, Out, M> + Send + 'static,
        M,
        Out,
        E: 'static,
        B: Bundle,
    > IntoObserverSystem<E, B, M, Out> for S
where
    S::System: ObserverSystem<E, B, Out>,
{
    type System = <S as IntoSystem<Trigger<'static, E, B>, Out, M>>::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<E: 'static, B: Bundle, Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<fn(Trigger<E, B>, $($param,)*)> for Func
        where
        for <'a> &'a mut Func:
                FnMut(Trigger<E, B>, $($param),*) -> Out +
                FnMut(Trigger<E, B>, $(SystemParamItem<$param>),*) -> Out, Out: 'static
        {
            type In = Trigger<'static, E, B>;
            type Out = Out;
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, input: Trigger<'static, E, B>, param_value: SystemParamItem< ($($param,)*)>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<E: 'static, B: Bundle, Out, $($param,)*>(
                    mut f: impl FnMut(Trigger<'static, E, B>, $($param,)*) -> Out,
                    input: Trigger<'static, E, B>,
                    $($param: $param,)*
                ) -> Out{
                    f(input, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, input, $($param),*)
            }
        }
    }
}

all_tuples!(impl_system_function, 0, 16, F);

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        event::Event,
        observer::Trigger,
        system::{In, IntoSystem},
        world::World,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    fn test_piped_observer_systems_no_input() {
        fn a(_: Trigger<TriggerEvent>) {}
        fn b() {}

        let mut world = World::new();
        world.observe(a.pipe(b));
    }

    #[test]
    fn test_piped_observer_systems_with_inputs() {
        fn a(_: Trigger<TriggerEvent>) -> u32 {
            3
        }
        fn b(_: In<u32>) {}

        let mut world = World::new();
        world.observe(a.pipe(b));
    }
}
