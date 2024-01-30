use crate::{
    prelude::Observer,
    query::{QueryData, QueryFilter},
    system::{System, SystemParam},
    world::DeferredWorld,
};

use bevy_utils::all_tuples;
#[cfg(feature = "trace")]
use bevy_utils::tracing::{info_span, Span};

use super::{
    Commands, FunctionSystem, IntoSystem, Query, Res, ResMut, Resource, SystemParamFunction,
};

pub trait ObserverSystem<E: 'static>:
    System<In = Observer<'static, E>, Out = ()> + Send + 'static
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
impl<E: 'static, Marker, F> ObserverSystem<E> for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker, In = Observer<'static, E>, Out = ()>,
    F::Param: ObserverSystemParam,
{
    fn queue_deferred(&mut self, world: DeferredWorld) {
        let param_state = self.param_state.as_mut().unwrap();
        F::Param::queue(param_state, &self.system_meta, world);
    }
}

pub trait IntoObserverSystem<E: 'static, M> {
    type System: ObserverSystem<E>;

    fn into_system(this: Self) -> Self::System;
}

impl<S: IntoSystem<Observer<'static, E>, (), M>, M, E: 'static> IntoObserverSystem<E, M> for S
where
    S::System: ObserverSystem<E>,
{
    type System = <S as IntoSystem<Observer<'static, E>, (), M>>::System;

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
