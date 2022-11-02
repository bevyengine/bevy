use crate::{
    prelude::{FromWorld, QueryState},
    query::{ReadOnlyWorldQuery, WorldQuery},
    system::{Local, LocalState, SystemMeta, SystemParam, SystemState},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use bevy_utils::synccell::SyncCell;

pub trait ExclusiveSystemParam: Sized {
    type Fetch: for<'s> ExclusiveSystemParamFetch<'s>;
}

pub type ExclusiveSystemParamItem<'s, P> =
    <<P as ExclusiveSystemParam>::Fetch as ExclusiveSystemParamFetch<'s>>::Item;

/// The state of a [`SystemParam`].
pub trait ExclusiveSystemParamState: Send + Sync {
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self;
    #[inline]
    fn apply(&mut self, _world: &mut World) {}
}

pub trait ExclusiveSystemParamFetch<'state>: ExclusiveSystemParamState {
    type Item: ExclusiveSystemParam<Fetch = Self>;
    fn get_param(state: &'state mut Self, system_meta: &SystemMeta) -> Self::Item;
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

    fn get_param(state: &'s mut Self, _system_meta: &SystemMeta) -> Self::Item {
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

    fn get_param(state: &'s mut Self, _system_meta: &SystemMeta) -> Self::Item {
        state
    }
}

impl<P: SystemParam> ExclusiveSystemParamState for SystemState<P> {
    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        SystemState::new(world)
    }
}

impl<'s, T: FromWorld + Send + Sync + 'static> ExclusiveSystemParam for Local<'s, T> {
    type Fetch = LocalState<T>;
}

impl<'s, T: FromWorld + Send + Sync + 'static> ExclusiveSystemParamFetch<'s> for LocalState<T> {
    type Item = Local<'s, T>;

    fn get_param(state: &'s mut Self, _system_meta: &SystemMeta) -> Self::Item {
        Local(state.0.get())
    }
}

impl<T: FromWorld + Send + Sync> ExclusiveSystemParamState for LocalState<T> {
    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self {
        Self(SyncCell::new(T::from_world(world)))
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
            ) -> Self::Item {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_meta),)*)
            }
        }

        // SAFETY: implementors of each `ExclusiveSystemParamState` in the tuple have validated their impls
        #[allow(clippy::undocumented_unsafe_blocks)] // false positive by clippy
        #[allow(non_snake_case)]
        impl<$($param: ExclusiveSystemParamState),*> ExclusiveSystemParamState for ($($param,)*) {
            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self {
                (($($param::init(_world, _system_meta),)*))
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
