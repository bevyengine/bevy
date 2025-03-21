use core::marker::PhantomData;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Tick, resource::Resource};
use bevy_platform_support::collections::HashMap;
use bevy_render::{render_resource::CachedRenderPipelineId, sync_world::MainEntityHashMap};

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
