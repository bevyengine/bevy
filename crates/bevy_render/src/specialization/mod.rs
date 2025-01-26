pub mod view;

use crate::extract_resource::ExtractResource;
use crate::render_resource::CachedRenderPipelineId;
use crate::specialization::view::{SpecializeViewKey, ViewSpecializationTicks};
use crate::sync_world::{MainEntity, MainEntityHashMap};
use crate::{Extract, ExtractSchedule, RenderApp};
use bevy_app::{App, Plugin};
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
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(ExtractSchedule, extract_entities_needs_specialization::<M>)
                .init_resource::<EntitySpecializationTicks<M>>()
                .init_resource::<SpecializedMaterialPipelineCache::<M>>();
        }
    }
}

#[derive(Clone, Resource, Debug)]
pub struct EntitySpecializationTicks<M> {
    pub entities: MainEntityHashMap<Tick>,
    _marker: PhantomData<M>,
}

impl <M> Default for EntitySpecializationTicks<M> {
    fn default() -> Self {
        Self {
            entities: MainEntityHashMap::default(),
            _marker: Default::default(),
        }
    }
}

#[derive(SystemParam)]
pub struct SpecializedPipelines<'w, M>
where
    M: NeedsSpecialization,
{
    entity_specialization_ticks: Res<'w, EntitySpecializationTicks<M>>,
    view_specialization_ticks: Res<'w, ViewSpecializationTicks<<M as NeedsSpecialization>::ViewKey>>,
    specialized_material_pipeline_cache: ResMut<'w, SpecializedMaterialPipelineCache<M>>,
    ticks: SystemChangeTick,
}

impl<M> SpecializedPipelines<'_, M>
where
    M: NeedsSpecialization,
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

    pub fn insert_pipeline(
        &mut self,
        view_entity: MainEntity,
        entity: MainEntity,
        pipeline_id: CachedRenderPipelineId,
    ) {
        self.specialized_material_pipeline_cache
            .insert((view_entity, entity), (self.ticks.this_run(), pipeline_id));
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
}

pub fn extract_entities_needs_specialization<M>(
    mut thread_queues: Local<Parallel<Vec<MainEntity>>>,
    mut needs_specialization: Extract<Query<(Entity, M::QueryData), M::QueryFilter>>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks<M>>,
    ticks: SystemChangeTick,
) where
    M: NeedsSpecialization,
{
    needs_specialization.par_iter_mut().for_each_init(
        || thread_queues.borrow_local_mut(),
        |queue, (entity, item)| {
            if M::needs_specialization(item) {
                println!("Entity {:?} needs specialization", entity);
                queue.push(entity.into());
            }
        },
    );

    let size = thread_queues.iter_mut().map(|queue| queue.len()).sum();
    entity_specialization_ticks.entities.reserve(size);
    let this_run = ticks.this_run();
    for queue in thread_queues.iter_mut() {
        for entity in queue.drain(..) {
            // Update the entity's specialization tick with this run's tick
            entity_specialization_ticks.entities.insert(entity, this_run);
        }
    }
}


pub trait NeedsSpecialization: Component {
    type ViewKey: SpecializeViewKey;
    type QueryData: ReadOnlyQueryData + 'static;
    type QueryFilter: QueryFilter + 'static;

    fn needs_specialization(
        item: ROQueryItem<'_, Self::QueryData>,
    ) -> bool;
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