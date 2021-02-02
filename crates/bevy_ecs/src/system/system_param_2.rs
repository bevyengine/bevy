use std::ops::{Deref, DerefMut};

use crate::{FromResources, Resources, SystemState, World};

/// System parameters which can be in exclusive systems (i.e. those which have `&mut World`, `&mut Resources` arguments)
/// This allows these systems to use local state
pub trait PureSystemParam: Sized + Send + Sync + 'static {
    type Config: Default;
    fn create_state_pure(config: Self::Config, resources: &mut Resources) -> Self::State;

    type State: for<'a> PureParamState<'a>;
    // For documentation purposes, at some future point
    // type Item = <Self::State as PurePureSystemState<'static>>::Item;
}

pub trait PureParamState<'a>: Sized + Send + Sync + 'static {
    type Item;
    fn view_param(&'a mut self) -> Self::Item;
}

#[derive(Debug)]
pub struct Local<T>(T);

impl<T> Deref for Local<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for Local<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// TODO: Equivalent impl for &Local<T> - would need type
impl<T: FromResources + 'static + Send + Sync> PureSystemParam for &'static mut Local<T> {
    type Config = Option<T>;
    type State = Local<T>;

    fn create_state_pure(config: Self::Config, resources: &mut Resources) -> Self::State {
        Local(config.unwrap_or_else(|| T::from_resources(resources)))
    }
}

impl<'a, T: Send + Sync + 'static> PureParamState<'a> for Local<T> {
    type Item = &'a mut Local<T>;

    fn view_param(&'a mut self) -> Self::Item {
        self
    }
}

pub trait SystemParam: Sized {
    type Config: Default;
    type State: for<'a> ParamState<'a>;
    fn create_state(config: Self::Config, resources: &mut Resources) -> Self::State;
}

pub trait ParamState<'a>: Send + Sync + 'static {
    type Item;

    unsafe fn get_param(
        &'a mut self,
        system_state: &'a SystemState,
        world: &'a World,
        resources: &'a Resources,
    ) -> Self::Item;
    // TODO: Make this significantly cleaner by having methods for resource access and archetype access
    // That is, don't store that information in SystemState
    fn init(&mut self, system_state: &mut SystemState, _world: &World, _resources: &mut Resources);
    // TODO: investigate `fn requires_sync()->bool{false}` to determine if there are pessimisations resulting from the lack thereof
    fn run_sync(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

// Any `PureSystemParam` can be an 'impure' `SystemParam`
impl<T: PureSystemParam> SystemParam for T {
    type Config = T::Config;

    type State = T::State;

    fn create_state(config: Self::Config, resources: &mut Resources) -> Self::State {
        T::create_state_pure(config, resources)
    }
}

impl<'a, T: PureParamState<'a>> ParamState<'a> for T {
    type Item = <T as PureParamState<'a>>::Item;

    unsafe fn get_param(
        &'a mut self,
        _: &'a crate::SystemState,
        _: &'a World,
        _: &'a Resources,
    ) -> Self::Item {
        self.view_param()
    }

    fn init(&mut self, _: &mut SystemState, _: &World, _: &mut Resources) {}
}
