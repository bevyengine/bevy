use core::marker::PhantomData;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Tick, entity::Entity, resource::Resource};
use bevy_platform_support::collections::HashMap;
use bevy_render::{render_resource::CachedRenderPipelineId, sync_world::MainEntityHashMap};

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

/// Stores the [`SpecializedMaterial2dViewPipelineCache`] for each view.
#[derive(Resource, Deref, DerefMut)]
pub struct SpecializedMaterial2dPipelineCache<M> {
    // view_entity -> view pipeline cache
    #[deref]
    map: MainEntityHashMap<SpecializedMaterial2dViewPipelineCache<M>>,
    marker: PhantomData<M>,
}

/// Stores the cached render pipeline ID for each entity in a single view, as
/// well as the last time it was changed.
#[derive(Deref, DerefMut)]
pub struct SpecializedMaterial2dViewPipelineCache<M> {
    // material entity -> (tick, pipeline_id)
    #[deref]
    map: MainEntityHashMap<(Tick, CachedRenderPipelineId)>,
    marker: PhantomData<M>,
}

impl<M> Default for SpecializedMaterial2dPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            marker: PhantomData,
        }
    }
}

impl<M> Default for SpecializedMaterial2dViewPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            marker: PhantomData,
        }
    }
}
