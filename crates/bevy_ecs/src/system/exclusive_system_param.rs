use crate::{
    prelude::{FromWorld, QueryState},
    query::{ReadOnlyWorldQuery, WorldQuery},
    system::{Local, SystemMeta, SystemParam, SystemState},
    world::World,
};
use bevy_utils::all_tuples;
use bevy_utils::synccell::SyncCell;

/// A parameter that can be used in an exclusive system (a system with an `&mut World` parameter).
/// Any parameters implementing this trait must come after the `&mut World` parameter.
pub trait ExclusiveSystemParam: Sized {
    /// Used to store data which persists across invocations of a system.
    type State: Send + Sync + 'static;
    /// The item type returned when constructing this system param.
    /// See [`SystemParam::Item`].
    type Item<'s>: ExclusiveSystemParam<State = Self::State>;

    /// Creates a new instance of this param's [`State`](Self::State).
    fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self::State;

    /// Creates a parameter to be passed into an [`ExclusiveSystemParamFunction`].
    ///
    /// [`ExclusiveSystemParamFunction`]: super::ExclusiveSystemParamFunction
    fn get_param<'s>(state: &'s mut Self::State, system_meta: &SystemMeta) -> Self::Item<'s>;
}

/// Shorthand way of accessing the associated type [`ExclusiveSystemParam::Item`]
/// for a given [`ExclusiveSystemParam`].
pub type ExclusiveSystemParamItem<'s, P> = <P as ExclusiveSystemParam>::Item<'s>;

impl<'a, Q: WorldQuery + 'static, F: ReadOnlyWorldQuery + 'static> ExclusiveSystemParam
    for &'a mut QueryState<Q, F>
{
    type State = QueryState<Q, F>;
    type Item<'s> = &'s mut QueryState<Q, F>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        QueryState::new(world)
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        state
    }
}

impl<'a, P: SystemParam + 'static> ExclusiveSystemParam for &'a mut SystemState<P> {
    type State = SystemState<P>;
    type Item<'s> = &'s mut SystemState<P>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SystemState::new(world)
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        state
    }
}

impl<'_s, T: FromWorld + Send + 'static> ExclusiveSystemParam for Local<'_s, T> {
    type State = SyncCell<T>;
    type Item<'s> = Local<'s, T>;

    fn init(world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(T::from_world(world))
    }

    fn get_param<'s>(state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        Local(state.get())
    }
}

macro_rules! impl_exclusive_system_param_tuple {
    ($($param: ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($param: ExclusiveSystemParam),*> ExclusiveSystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'s> = ($($param::Item<'s>,)*);

            #[inline]
            fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
                (($($param::init(_world, _system_meta),)*))
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            fn get_param<'s>(
                state: &'s mut Self::State,
                system_meta: &SystemMeta,
            ) -> Self::Item<'s> {

                let ($($param,)*) = state;
                ($($param::get_param($param, system_meta),)*)
            }
        }
    };
}

all_tuples!(impl_exclusive_system_param_tuple, 0, 16, P);
