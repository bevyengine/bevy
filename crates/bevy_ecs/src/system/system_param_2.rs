use crate::{FromResources, Resources, World};

/// System parameters which can be in exclusive systems (i.e. those which have `&mut World`, `&mut Resources` arguments)
/// This allows these systems to use local state
pub trait PureSystemParam: Sized {
    type Config;
    fn create_state(config: Self::Config, resources: &mut Resources) -> Self::State;

    type State: for<'a> PureSystemState<'a>;
    // For documentation purposes, at some future point
    // type Item = <Self::State as PurePureSystemState<'static>>::Item;
}

pub trait PureSystemState<'a> {
    type Item;
    fn get_param(&'a mut self) -> Self::Item;
}

#[derive(Debug)]
struct Local<T>(T);

// TODO: Equivalent impl for &Local<T> - would need type
impl<T: FromResources + 'static> PureSystemParam for &mut Local<T> {
    type Config = Option<T>;
    type State = Local<T>;

    fn create_state(config: Self::Config, resources: &mut Resources) -> Self::State {
        Local(config.unwrap_or_else(|| T::from_resources(resources)))
    }
}

impl<'a, T: 'static> PureSystemState<'a> for Local<T> {
    type Item = &'a mut Local<T>;

    fn get_param(&'a mut self) -> Self::Item {
        self
    }
}

pub trait SystemParam: Sized {
    type Config;
    type State: for<'a> SystemState<'a>;
    fn create_state(config: Self::Config, resources: &mut Resources) -> Self::State;
}

pub trait SystemState<'a> {
    type Item;

    fn init();
    fn get_param(
        &'a mut self,
        system_state: &'a crate::SystemState,
        world: &'a World,
        resources: &'a Resources,
    ) -> Self::Item;
}

// Any `PureSystemParam` can be an 'impure' `SystemParam`
#[doc(hidden)]
pub struct PureParamState<T>(T);

impl<T: PureSystemParam> SystemParam for T {
    type Config = T::Config;

    type State = PureParamState<T::State>;

    fn create_state(config: Self::Config, resources: &mut Resources) -> Self::State {
        PureParamState(T::create_state(config, resources))
    }
}

impl<'a, T: PureSystemState<'a>> SystemState<'a> for PureParamState<T> {
    type Item = <T as PureSystemState<'a>>::Item;

    fn get_param(
        &'a mut self,
        _: &'a crate::SystemState,
        _: &'a World,
        _: &'a Resources,
    ) -> Self::Item {
        T::get_param(&mut self.0)
    }

    fn init() {
        todo!()
    }
}
