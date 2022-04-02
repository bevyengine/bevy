use std::ops::{Deref, DerefMut};

use crate::MainWorld;
use bevy_ecs::{
    prelude::*,
    system::{
        ReadOnlySystemParamFetch, SystemParam, SystemParamFetch, SystemParamItem, SystemParamState,
        SystemState,
    },
};

pub struct ExtractFromMainWorld<'s, P: SystemParam + 'static>(SystemParamItem<'s, 's, P>)
where
    P::Fetch: ReadOnlySystemParamFetch;

impl<'s, P: SystemParam + 'static> Deref for ExtractFromMainWorld<'s, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    type Target = SystemParamItem<'s, 's, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'s, P: SystemParam + 'static> DerefMut for ExtractFromMainWorld<'s, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'s, P: SystemParam + 'static> ExtractFromMainWorld<'s, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    pub fn into_inner(self) -> SystemParamItem<'s, 's, P> {
        self.0
    }
}

impl<'s, P: SystemParam + 'static> SystemParam for ExtractFromMainWorld<'s, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    type Fetch = ExtractFromMainWorldState<P>;
}

unsafe impl<P: SystemParam + 'static> SystemParamState for ExtractFromMainWorldState<P> {
    fn init(world: &mut World, system_meta: &mut bevy_ecs::system::SystemMeta) -> Self {
        Self {
            world: SystemParamState::init(world, system_meta),
            state: SystemState::new(&mut (*world.resource_mut::<MainWorld>())),
        }
    }
}

pub struct ExtractFromMainWorldState<P: SystemParam + 'static> {
    world: <Res<'static, MainWorld> as SystemParam>::Fetch,
    state: SystemState<P>,
}

impl<'world, 'state, P: SystemParam + 'static> SystemParamFetch<'world, 'state>
    for ExtractFromMainWorldState<P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    type Item = ExtractFromMainWorld<'state, P>;

    unsafe fn get_param(
        state: &'state mut Self,
        system_meta: &bevy_ecs::system::SystemMeta,
        world: &'world World,
        change_tick: u32,
    ) -> Self::Item {
        let world = SystemParamFetch::get_param(&mut state.world, system_meta, world, change_tick);
        ExtractFromMainWorld(state.state.get(&world))
    }
}
