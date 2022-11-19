use crate::{
    prelude::{FromWorld, QueryState},
    query::{ReadOnlyWorldQuery, WorldQuery},
    system::{Local, LocalState, SystemMeta, SystemParam, SystemState},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use bevy_utils::synccell::SyncCell;

pub trait ExclusiveSystemParam: Sized {
    type State: ExclusiveSystemParamState;
}

pub type ExclusiveSystemParamItem<'s, P> =
    <<P as ExclusiveSystemParam>::State as ExclusiveSystemParamState>::Item<'s>;

/// The state of a [`SystemParam`].
pub trait ExclusiveSystemParamState: Send + Sync + 'static {
    type Item<'s>: ExclusiveSystemParam<State = Self>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self;
    #[inline]
    fn apply(&mut self, _world: &mut World) {}

    fn get_param<'s>(state: &'s mut Self, system_meta: &SystemMeta) -> Self::Item<'s>;
}

impl<'a, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParam
    for &'a mut QueryState<Q, F>
{
    type State = QueryState<Q, F>;
}

impl<Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParamState
    for QueryState<Q, F>
{
    type Item<'s> = &'s mut QueryState<Q, F>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        QueryState::new(world)
    }

    fn get_param<'s>(state: &'s mut Self, _system_meta: &SystemMeta) -> Self::Item<'s> {
        state
    }
}

impl<'a, P: SystemParam + 'static> ExclusiveSystemParam for &'a mut SystemState<P> {
    type State = SystemState<P>;
}

impl<P: SystemParam> ExclusiveSystemParamState for SystemState<P> {
    type Item<'s> = &'s mut SystemState<P>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        SystemState::new(world)
    }

    fn get_param<'s>(state: &'s mut Self, _system_meta: &SystemMeta) -> Self::Item<'s> {
        state
    }
}

impl<'s, T: FromWorld + Send + Sync + 'static> ExclusiveSystemParam for Local<'s, T> {
    type State = LocalState<T>;
}

impl<T: FromWorld + Send + Sync> ExclusiveSystemParamState for LocalState<T> {
    type Item<'s> = Local<'s, T>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self(SyncCell::new(T::from_world(world)))
    }

    fn get_param<'s>(state: &'s mut Self, _system_meta: &SystemMeta) -> Self::Item<'s> {
        Local(state.0.get())
    }
}

macro_rules! impl_exclusive_system_param_tuple {
    ($($param: ident),*) => {
        impl<$($param: ExclusiveSystemParam),*> ExclusiveSystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($param: ExclusiveSystemParamState),*> ExclusiveSystemParamState for ($($param,)*) {
            type Item<'s> = ($($param::Item<'s>,)*);

            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
                (($($param::init(_world, _system_meta),)*))
            }

            #[inline]
            fn apply(&mut self, _world: &mut World) {
                let ($($param,)*) = self;
                $($param.apply(_world);)*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn get_param<'s>(
                state: &'s mut Self,
                system_meta: &SystemMeta,
            ) -> Self::Item<'s> {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_meta),)*)
            }
        }

    };
}

all_tuples!(impl_exclusive_system_param_tuple, 0, 16, P);
