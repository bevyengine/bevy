use std::ops::{Deref, DerefMut};

use crate::{Or, Resources, SystemState, World};

mod impls;

/// System parameters which can be in exclusive systems (i.e. those which have `&mut World`, `&mut Resources` arguments)
/// This allows these systems to use local state
pub trait PureSystemParam: Sized {
    type PureConfig: 'static;
    fn create_state_pure(config: Self::PureConfig) -> Self::PureState;

    type PureState: for<'a> PureParamState<'a>;
    // For documentation purposes, at some future point
    // type Item = <Self::State as PurePureSystemState<'static>>::Item;
    fn default_config_pure() -> Self::PureConfig;
}

pub trait PureParamState<'a>: Sized + Send + Sync + 'static {
    type Item;
    fn view_param(&'a mut self) -> Self::Item;
}

pub trait SystemParam: Sized {
    type Config: 'static;
    type State: for<'a> ParamState<'a> + 'static;
    fn create_state(config: Self::Config) -> Self::State;
    fn default_config() -> Self::Config;
}

pub trait ParamState<'a>: Sized + Send + Sync + 'static {
    type Item;

    /// # Safety
    ///
    /// Init must have been called and the aliasing requirements set up in
    /// SystemState must be met
    unsafe fn get_param(
        &'a mut self,
        system_state: &'a SystemState,
        world: &'a World,
        resources: &'a Resources,
    ) -> Option<Self::Item>;
    // TODO: Make this significantly cleaner by having methods for resource access and archetype access
    // That is, don't store that information in SystemState
    fn init(&mut self, system_state: &mut SystemState, world: &World, resources: &mut Resources);
    // TODO: investigate `fn requires_sync()->bool{false}` to determine if there are pessimisations resulting from the lack thereof
    fn run_sync(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

// TODO: This impl is too clever - in particular it breaks being able to use tuples of PureSystemParam
// Any `PureSystemParam` can be an 'impure' `SystemParam`
impl<T: PureSystemParam> SystemParam for T {
    type Config = T::PureConfig;

    type State = T::PureState;

    fn create_state(config: Self::Config) -> Self::State {
        T::create_state_pure(config)
    }

    fn default_config() -> Self::Config {
        T::default_config_pure()
    }
}

// Is this impl also too clever? Probably is so
impl<'a, T: PureParamState<'a>> ParamState<'a> for T {
    type Item = <T as PureParamState<'a>>::Item;

    unsafe fn get_param(
        &'a mut self,
        _: &'a crate::SystemState,
        _: &'a World,
        _: &'a Resources,
    ) -> Option<Self::Item> {
        Some(self.view_param())
    }

    fn init(&mut self, _: &mut SystemState, _: &World, _: &mut Resources) {}
}

#[derive(Debug)]
pub struct Local<'a, T>(pub(crate) &'a mut T);

impl<'a, T> DerefMut for Local<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a, T> Deref for Local<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

macro_rules! impl_system_param_tuple {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        #[allow(unused_variables)] // Unused in the (,) case
        impl<$($param: SystemParam,)*> SystemParam for ($($param,)*) {
            type Config = ($($param::Config,)*);
            type State = ($($param::State,)*);

            fn create_state(config: Self::Config) -> Self::State {
                let ($($param,)*) = config;
                ($($param::create_state($param),)*)
            }

            fn default_config() -> Self::Config{
                ($($param::default_config(),)*)
            }
        }
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'a, $($param: ParamState<'a>,)*> ParamState<'a> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            unsafe fn get_param(
                &'a mut self,
                system_state: &'a SystemState,
                world: &'a World,
                resources: &'a Resources,
            ) -> Option<Self::Item> {
                let ($($param,)*) = self;
                Some((
                    $($param.get_param(
                        system_state,
                        world,
                        resources
                    )?,)*
                ))
            }

            fn init(
                &mut self,
                system_state: &mut SystemState,
                world: &World,
                resources: &mut Resources,
            ) {
                let ($($param,)*) = self;
                $($param.init(system_state, world, resources);)*
            }

            fn run_sync(&mut self, world: &mut World, resources: &mut Resources) {
                let ($($param,)*) = self;
                $($param.run_sync(world, resources);)*
            }
        }

        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        impl<$($param: SystemParam,)*> SystemParam for Or<($(Option<$param>,)*)> {
            type Config = ($($param::Config,)*);
            type State = Or<($($param::State,)*)>;

            fn create_state(config: Self::Config) -> Self::State {
                let ($($param,)*) = config;
                Or(($($param::create_state($param),)*))
            }
            fn default_config() -> Self::Config{
                ($($param::default_config(),)*)
            }
        }

        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        impl<'a, $($param: ParamState<'a>,)*> ParamState<'a> for Or<($($param,)*)> {
            type Item = ($(Option<$param::Item>,)*);

            unsafe fn get_param(
                &'a mut self,
                system_state: &'a SystemState,
                world: &'a World,
                resources: &'a Resources,
            ) -> Option<Self::Item> {
                // Required for the (,) case
                #[allow(unused_mut)]
                let mut has_some = false;
                let ($($param,)*) = &mut self.0;

                $(
                    let $param = $param.get_param(system_state, world, resources);
                    if $param.is_some() {
                        has_some = true;
                    }
                )*

                let v = ($($param,)*);
                if has_some {
                    Some(v)
                } else {
                    None
                }
            }

            fn init(
                &mut self,
                system_state: &mut SystemState,
                world: &World,
                resources: &mut Resources,
            ) {
                let ($($param,)*) = &mut self.0;
                $($param.init(system_state, world, resources);)*
            }

            fn run_sync(&mut self, world: &mut World, resources: &mut Resources) {
                let ($($param,)*) = &mut self.0;
                $($param.run_sync(world, resources);)*
            }
        }
    };
}

impl_system_param_tuple!();
impl_system_param_tuple!(T1);
impl_system_param_tuple!(T1, T2);
impl_system_param_tuple!(T1, T2, T3);
impl_system_param_tuple!(T1, T2, T3, T4);
impl_system_param_tuple!(T1, T2, T3, T4, T5);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

// We can't use default because these use more types than tuples
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_system_param_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
