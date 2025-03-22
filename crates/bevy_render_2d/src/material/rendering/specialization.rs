use core::marker::PhantomData;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Tick, entity::Entity, resource::Resource};
use bevy_render::sync_world::MainEntityHashMap;

#[derive(Clone, Resource, Deref, DerefMut, Debug)]
pub struct EntitiesNeedingSpecialization<M> {
    #[deref]
    pub entities: Vec<Entity>,
    _marker: PhantomData<M>,
}

impl<M> Default for EntitiesNeedingSpecialization<M> {
    fn default() -> Self {
        Self {
            entities: Default::default(),
            _marker: Default::default(),
        }
    }
}

#[derive(Clone, Resource, Deref, DerefMut, Debug)]
pub struct EntitySpecializationTicks<M> {
    #[deref]
    pub entities: MainEntityHashMap<Tick>,
    _marker: PhantomData<M>,
}

impl<M> Default for EntitySpecializationTicks<M> {
    fn default() -> Self {
        Self {
            entities: MainEntityHashMap::default(),
            _marker: Default::default(),
        }
    }
}
