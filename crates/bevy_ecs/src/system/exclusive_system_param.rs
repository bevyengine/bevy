use crate::{
    prelude::{FromWorld, QueryState},
    query::{ReadOnlyWorldQuery, WorldQuery},
    system::{Local, SystemMeta, SystemParam, SystemState},
    world::World,
};
use bevy_utils::all_tuples;
use bevy_utils::synccell::SyncCell;

pub trait ExclusiveSystemParam: Sized {
    type State: Send + Sync + 'static;
    type Item<'state>: ExclusiveSystemParam<State = Self::State>;

    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self::State;

    fn get_param<'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
    ) -> Self::Item<'state>;
}

pub type ExclusiveSystemParamItem<'state, P> = <P as ExclusiveSystemParam>::Item<'state>;

impl<'a, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParam
    for &'a mut QueryState<Q, F>
{
    type State = QueryState<Q, F>;
    type Item<'state> = &'state mut QueryState<Q, F>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        QueryState::new(world)
    }

    fn get_param<'state>(
        state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'state> {
        state
    }
}

impl<'a, P: SystemParam + 'static> ExclusiveSystemParam for &'a mut SystemState<P> {
    type State = SystemState<P>;
    type Item<'state> = &'state mut SystemState<P>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SystemState::new(world)
    }

    fn get_param<'state>(
        state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'state> {
        state
    }
}

impl<'_s, T: FromWorld + Send + 'static> ExclusiveSystemParam for Local<'_s, T> {
    type State = SyncCell<T>;
    type Item<'state> = Local<'state, T>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(T::from_world(world))
    }

    fn get_param<'state>(
        state: &'state mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'state> {
        Local(state.get())
    }
}

macro_rules! impl_exclusive_system_param_tuple {
    ($($param: ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($param: ExclusiveSystemParam),*> ExclusiveSystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'state> = ($($param::Item<'state>,)*);

            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
                (($($param::init(_world, _system_meta),)*))
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn get_param<'state>(
                state: &'state mut Self::State,
                system_meta: &SystemMeta,
            ) -> Self::Item<'state> {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_meta),)*)
            }
        }
    };
}

all_tuples!(impl_exclusive_system_param_tuple, 0, 16, P);
