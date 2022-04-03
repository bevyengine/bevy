use crate::MainWorld;
use bevy_ecs::{
    prelude::*,
    system::{
        ReadOnlySystemParamFetch, SystemParam, SystemParamFetch, SystemParamItem, SystemState,
    },
};

pub struct MainWorldState<P: SystemParam>(SystemState<P>);

impl<P: SystemParam> FromWorld for MainWorldState<P> {
    fn from_world(world: &mut World) -> Self {
        Self(SystemState::new(&mut world.resource_mut::<MainWorld>().0))
    }
}

#[derive(SystemParam)]
pub struct Extract<'w, 's, P: SystemParam + 'static>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    state: Local<
        's,
        MainWorldState<<<P as SystemParam>::Fetch as SystemParamFetch<'static, 'static>>::Item>,
    >,
    world: Res<'w, World>,
}

impl<'w, 's, P: SystemParam + 'static> Extract<'w, 's, P>
where
    P::Fetch: ReadOnlySystemParamFetch,
{
    pub fn value(&mut self) -> SystemParamItem<'_, '_, P> {
        self.state.0.get(&self.world)
    }
}
