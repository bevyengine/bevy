pub mod view;

use crate::extract_resource::ExtractResource;
use crate::mesh::RenderMesh;
use crate::render_resource::{
    CachedRenderPipelineId, PipelineCache, SpecializedMeshPipeline, SpecializedMeshPipelines,
};
use crate::specialization::view::{GetViewKey, ViewSpecializationTicks};
use crate::sync_world::{MainEntity, MainEntityHashMap};
use crate::{Extract, ExtractSchedule, RenderApp};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::AssetEvents;
use bevy_ecs::component::{Component, Tick};
use bevy_ecs::entity::{Entity, EntityBorrow, EntityHash};
use bevy_ecs::prelude::SystemSet;
use bevy_ecs::query::{QueryFilter, ROQueryItem, ReadOnlyQueryData};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::schedule::IntoSystemSetConfigs;
use bevy_ecs::system::{Local, Query, Res, ResMut, SystemChangeTick, SystemParam};
use bevy_platform_support::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_utils::Parallel;
use core::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use tracing::error;

pub struct CheckSpecializationPlugin<M>(PhantomData<M>);

impl<M> Default for CheckSpecializationPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M> Plugin for CheckSpecializationPlugin<M>
where
    M: NeedsSpecialization,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            check_entities_needing_specialization::<M>.after(AssetEvents),
        )
        .init_resource::<EntitiesNeedingSpecialization<M>>();
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(ExtractSchedule, extract_entities_needs_specialization::<M>)
                .init_resource::<EntitySpecializationTicks<M>>()
                .init_resource::<SpecializedMaterialPipelineCache<M>>();
        }
    }
}

pub trait NeedsSpecialization: Component {
    type ViewKey: GetViewKey;
    type Pipeline: SpecializedMeshPipeline + Resource + Send + Sync + 'static;
    type QueryData: ReadOnlyQueryData + 'static;
    type QueryFilter: QueryFilter + 'static;

    fn needs_specialization(item: ROQueryItem<'_, Self::QueryData>) -> bool;
}


#[derive(SystemParam)]
pub struct SpecializePipelines<'w, M>
where
    M: NeedsSpecialization,
    <<M as NeedsSpecialization>::Pipeline as SpecializedMeshPipeline>::Key: Send + Sync + 'static
{
    entity_specialization_ticks: Res<'w, EntitySpecializationTicks<M>>,
    view_specialization_ticks:
        Res<'w, ViewSpecializationTicks<<M as NeedsSpecialization>::ViewKey>>,
    specialized_material_pipeline_cache: ResMut<'w, SpecializedMaterialPipelineCache<M>>,
    pipelines: ResMut<'w, SpecializedMeshPipelines<<M as NeedsSpecialization>::Pipeline>>,
    pipeline: Res<'w, <M as NeedsSpecialization>::Pipeline>,
    pipeline_cache: Res<'w, PipelineCache>,
    ticks: SystemChangeTick,
}

impl<M> SpecializePipelines<'_, M>
where
    M: NeedsSpecialization,
    <<M as NeedsSpecialization>::Pipeline as SpecializedMeshPipeline>::Key: Send + Sync + 'static
{
    pub fn needs_specialization(&self, view_entity: MainEntity, entity: MainEntity) -> bool {
        let view_tick = self
            .view_specialization_ticks
            .entities
            .get(&view_entity)
            .expect("View entity not found in specialization ticks");
        let entity_tick = self
            .entity_specialization_ticks
            .entities
            .get(&entity)
            .expect("Entity not found in specialization ticks");
        let Some((last_specialized_tick, _)) = self
            .specialized_material_pipeline_cache
            .get(&(view_entity, entity))
        else {
            return true;
        };

        view_tick.is_newer_than(*last_specialized_tick, self.ticks.this_run())
            || entity_tick.is_newer_than(*last_specialized_tick, self.ticks.this_run())
    }
    pub fn get_pipeline(
        &self,
        view_entity: MainEntity,
        entity: MainEntity,
    ) -> Option<CachedRenderPipelineId> {
        self.specialized_material_pipeline_cache
            .get(&(view_entity, entity))
            .map(|(_, pipeline_id)| *pipeline_id)
    }

    pub fn specialize_pipeline(
        &mut self,
        (view_entity, visible_entity): (MainEntity, MainEntity),
        key: <<M as NeedsSpecialization>::Pipeline as SpecializedMeshPipeline>::Key,
        mesh: &RenderMesh,
    ) {
        let pipeline_id =
            self.pipelines
                .specialize(&self.pipeline_cache, &self.pipeline, key, &mesh.layout);
        let pipeline_id = match pipeline_id {
            Ok(id) => id,
            Err(err) => {
                error!("{}", err);
                return;
            }
        };

        self.specialized_material_pipeline_cache.insert(
            (view_entity, visible_entity),
            (self.ticks.this_run(), pipeline_id),
        );
    }
}

fn check_entities_needing_specialization<M>(
    mut thread_queues: Local<Parallel<Vec<Entity>>>,
    mut needs_specialization: Query<(Entity, M::QueryData), M::QueryFilter>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
) where
    M: NeedsSpecialization,
{
    entities_needing_specialization.entities.clear();
    needs_specialization.par_iter_mut().for_each_init(
        || thread_queues.borrow_local_mut(),
        |queue, (entity, item)| {
            if M::needs_specialization(item) {
                queue.push(entity.into());
            }
        },
    );

    thread_queues.drain_into(&mut entities_needing_specialization.entities);
}

pub fn extract_entities_needs_specialization<M>(
    mut entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<M>>>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks<M>>,
    ticks: SystemChangeTick,
) where
    M: NeedsSpecialization,
{
    for entity in &entities_needing_specialization.entities {
        // Update the entity's specialization tick with this run's tick
        entity_specialization_ticks
            .entities
            .insert((*entity).into(), ticks.this_run());
    }
}

#[derive(Clone, Resource, Debug)]
pub struct EntitiesNeedingSpecialization<M> {
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

#[derive(Clone, Resource, Debug)]
pub struct EntitySpecializationTicks<M> {
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

#[derive(Resource)]
pub struct SpecializedMaterialPipelineCache<M> {
    map: HashMap<(MainEntity, MainEntity), (Tick, CachedRenderPipelineId), EntityHash>,
    marker: PhantomData<M>,
}

impl<M> Default for SpecializedMaterialPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            marker: PhantomData,
        }
    }
}

impl<M> Deref for SpecializedMaterialPipelineCache<M> {
    type Target = HashMap<(MainEntity, MainEntity), (Tick, CachedRenderPipelineId), EntityHash>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<M> DerefMut for SpecializedMaterialPipelineCache<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
