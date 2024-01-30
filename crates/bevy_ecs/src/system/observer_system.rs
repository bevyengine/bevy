use crate::{
    prelude::{Bundle, Observer},
    query::{QueryData, QueryFilter},
    system::{System, SystemParam, SystemParamFunction, SystemParamItem},
    world::DeferredWorld,
};

use bevy_utils::all_tuples;
#[cfg(feature = "trace")]
use bevy_utils::tracing::{info_span, Span};

use super::{Commands, FunctionSystem, IntoSystem, Query, Res, ResMut, Resource};

pub trait ObserverSystem<E: 'static, B: Bundle>:
    System<In = Observer<'static, E, B>, Out = ()> + Send + 'static
{
    fn queue_deferred(&mut self, _world: DeferredWorld);
}

pub trait ObserverSystemParam: SystemParam {}

impl<'w, D: QueryData + 'static, F: QueryFilter + 'static> ObserverSystemParam
    for Query<'w, 'w, D, F>
{
}

impl<'w, T: Resource> ObserverSystemParam for Res<'w, T> {}

impl<'w, T: Resource> ObserverSystemParam for ResMut<'w, T> {}

impl<'w> ObserverSystemParam for Commands<'w, 'w> {}

/// SAFETY: `F`'s param is [`ReadOnlySystemParam`], so this system will only read from the world.
impl<E: 'static, B: Bundle, Marker, F> ObserverSystem<E, B> for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker, In = Observer<'static, E, B>, Out = ()>,
    F::Param: ObserverSystemParam,
{
    fn queue_deferred(&mut self, world: DeferredWorld) {
        let param_state = self.param_state.as_mut().unwrap();
        F::Param::queue(param_state, &self.system_meta, world);
    }
}

pub trait IntoObserverSystem<E: 'static, B: Bundle, M> {
    type System: ObserverSystem<E, B>;

    fn into_system(this: Self) -> Self::System;
}

impl<S: IntoSystem<Observer<'static, E, B>, (), M>, M, E: 'static, B: Bundle>
    IntoObserverSystem<E, B, M> for S
where
    S::System: ObserverSystem<E, B>,
{
    type System = <S as IntoSystem<Observer<'static, E, B>, (), M>>::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

macro_rules! impl_observer_system_param_tuple {
    ($($param: ident),*) => {
        #[allow(clippy::undocumented_unsafe_blocks)]
        #[allow(non_snake_case)]
        impl<$($param: ObserverSystemParam),*> ObserverSystemParam for ($($param,)*) {
        }
    };
}

all_tuples!(impl_observer_system_param_tuple, 0, 16, P);

macro_rules! impl_system_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<E: 'static, B: Bundle, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<fn(Observer<E, B>, $($param,)*)> for Func
        where
        for <'a> &'a mut Func:
                FnMut(Observer<E, B>, $($param),*) +
                FnMut(Observer<E, B>, $(SystemParamItem<$param>),*)
        {
            type In = Observer<'static, E, B>;
            type Out = ();
            type Param = ($($param,)*);
            #[inline]
            fn run(&mut self, input: Observer<'static, E, B>, param_value: SystemParamItem< ($($param,)*)>) {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<E: 'static, B: Bundle, $($param,)*>(
                    mut f: impl FnMut(Observer<'static, E, B>, $($param,)*),
                    input: Observer<'static, E, B>,
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
