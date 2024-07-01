use bevy_utils::all_tuples;

use crate::{
    prelude::{Bundle, Trigger},
    system::{System, SystemParam, SystemParamFunction, SystemParamItem},
};

use super::IntoSystem;

/// Implemented for systems that have an [`Observer`] as the first argument.
pub trait ObserverSystem<E: 'static, B: Bundle>:
    System<In = Trigger<'static, E, B>, Out = ()> + Send + 'static
{
}

impl<E: 'static, B: Bundle, T: System<In = Trigger<'static, E, B>, Out = ()> + Send + 'static>
    ObserverSystem<E, B> for T
{
}

/// Implemented for systems that convert into [`ObserverSystem`].
pub trait IntoObserverSystem<E: 'static, B: Bundle, M>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

impl<S: IntoSystem<Trigger<'static, E, B>, (), M> + Send + 'static, M, E: 'static, B: Bundle>
    IntoObserverSystem<E, B, M> for S
where
    S::System: ObserverSystem<E, B>,
{
    type System = <S as IntoSystem<Trigger<'static, E, B>, (), M>>::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<E: 'static, B: Bundle, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<fn(Trigger<E, B>, $($param,)*)> for Func
        where
        for <'a> &'a mut Func:
                FnMut(Trigger<E, B>, $($param),*) +
                FnMut(Trigger<E, B>, $(SystemParamItem<$param>),*)
        {
            type In = Trigger<'static, E, B>;
            type Out = ();
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, input: Trigger<'static, E, B>, param_value: SystemParamItem< ($($param,)*)>) {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<E: 'static, B: Bundle, $($param,)*>(
                    mut f: impl FnMut(Trigger<'static, E, B>, $($param,)*),
                    input: Trigger<'static, E, B>,
                    $($param: $param,)*
                ){
                    f(input, $($param,)*)
                }
                let ($($param,)*) = param_value;
                call_inner(self, input, $($param),*)
            }
        }
    }
}

all_tuples!(impl_system_function, 0, 16, F);
