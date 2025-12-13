use crate::{
    prelude::{FromWorld, QueryState},
    query::{QueryData, QueryFilter},
    system::{Local, SystemMeta, SystemParam, SystemState},
    world::World,
};
use bevy_platform::cell::SyncCell;
use core::marker::PhantomData;
use variadics_please::all_tuples;

/// A parameter that can be used in an exclusive system (a system with an `&mut World` parameter).
/// Any parameters implementing this trait must come after the `&mut World` parameter.
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not be used as a parameter for an exclusive system",
    label = "invalid system parameter"
)]
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

impl<'a, D: QueryData + 'static, F: QueryFilter + 'static> ExclusiveSystemParam
    for &'a mut QueryState<D, F>
{
    type State = QueryState<D, F>;
    type Item<'s> = &'s mut QueryState<D, F>;

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

impl<S: ?Sized> ExclusiveSystemParam for PhantomData<S> {
    type State = ();
    type Item<'s> = PhantomData<S>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {}

    fn get_param<'s>(_state: &'s mut Self::State, _system_meta: &SystemMeta) -> Self::Item<'s> {
        PhantomData
    }
}

macro_rules! impl_exclusive_system_param_tuple {
    ($(#[$meta:meta])* $($param: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is within a macro, and as such, the below lints may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "Certain variable names are provided by the caller, not by us."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        impl<$($param: ExclusiveSystemParam),*> ExclusiveSystemParam for ($($param,)*) {
            type State = ($($param::State,)*);
            type Item<'s> = ($($param::Item<'s>,)*);

            #[inline]
            fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
                (($($param::init(world, system_meta),)*))
            }

            #[inline]
            fn get_param<'s>(
                state: &'s mut Self::State,
                system_meta: &SystemMeta,
            ) -> Self::Item<'s> {
                let ($($param,)*) = state;
                #[allow(
                    clippy::unused_unit,
                    reason = "Zero-length tuples won't have any params to get."
                )]
                ($($param::get_param($param, system_meta),)*)
            }
        }
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_exclusive_system_param_tuple,
    0,
    16,
    P
);

#[cfg(test)]
mod tests {
    use crate::{schedule::Schedule, system::Local, world::World};
    use alloc::vec::Vec;
    use bevy_ecs_macros::Resource;
    use core::marker::PhantomData;

    #[test]
    fn test_exclusive_system_params() {
        #[derive(Resource, Default)]
        struct Res {
            test_value: u32,
        }

        fn my_system(world: &mut World, mut local: Local<u32>, _phantom: PhantomData<Vec<u32>>) {
            assert_eq!(world.resource::<Res>().test_value, *local);
            *local += 1;
            world.resource_mut::<Res>().test_value += 1;
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(my_system);

        let mut world = World::default();
        world.init_resource::<Res>();

        schedule.run(&mut world);
        schedule.run(&mut world);

        assert_eq!(2, world.get_resource::<Res>().unwrap().test_value);
    }
}
