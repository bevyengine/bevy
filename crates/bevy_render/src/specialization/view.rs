use crate::sync_world::{MainEntity, MainEntityHashMap};
use crate::Render;
use crate::RenderSet::PrepareAssets;
use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Tick;
use bevy_ecs::entity::hash_map::EntityHashMap;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;
use bevy_ecs::query::{QueryItem, ReadOnlyQueryData};
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::system::{Query, ResMut, SystemChangeTick};
use bevy_render_macros::ExtractResource;
use core::marker::PhantomData;

pub struct SpecializeViewsPlugin<VK>(PhantomData<VK>);

impl<VK> Plugin for SpecializeViewsPlugin<VK>
where
    VK: GetViewKey,
{
    fn build(&self, app: &mut App) {}

    fn finish(&self, app: &mut App) {
        app.add_systems(
            Render,
            check_views_need_specialization::<VK>.in_set(PrepareAssets),
        )
        .init_resource::<ViewKeyCache<VK>>()
        .init_resource::<ViewSpecializationTicks<VK>>();
    }
}

impl<VK> Default for SpecializeViewsPlugin<VK> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Resource, Deref, DerefMut, ExtractResource, Clone)]
pub struct ViewKeyCache<VK>(MainEntityHashMap<VK>)
where
    VK: GetViewKey;

impl<VK> Default for ViewKeyCache<VK>
where
    VK: GetViewKey,
{
    fn default() -> Self {
        Self(MainEntityHashMap::default())
    }
}

#[derive(Clone, Resource, Debug)]
pub struct ViewSpecializationTicks<VK> {
    pub entities: MainEntityHashMap<Tick>,
    _marker: PhantomData<VK>,
}

impl<VK> Default for ViewSpecializationTicks<VK> {
    fn default() -> Self {
        Self {
            entities: MainEntityHashMap::default(),
            _marker: PhantomData,
        }
    }
}

pub trait GetViewKey: PartialEq + Send + Sync + 'static {
    type QueryData: ReadOnlyQueryData + 'static;

    fn get_view_key<'w>(view_query: QueryItem<'w, Self::QueryData>) -> Self;
}

pub fn check_views_need_specialization<VK>(
    mut view_key_cache: ResMut<ViewKeyCache<VK>>,
    mut view_specialization_ticks: ResMut<ViewSpecializationTicks<VK>>,
    mut views: Query<(&MainEntity, VK::QueryData)>,
    ticks: SystemChangeTick,
) where
    VK: GetViewKey,
{
    for (view_entity, view_query) in views.iter_mut() {
        let view_key = VK::get_view_key(view_query);
        if let Some(current_key) = view_key_cache.get_mut(view_entity) {
            if *current_key != view_key {
                view_key_cache.insert(*view_entity, view_key);
                view_specialization_ticks
                    .entities
                    .insert(*view_entity, ticks.this_run());
            }
        } else {
            view_key_cache.insert(*view_entity, view_key);
            view_specialization_ticks
                .entities
                .insert(*view_entity, ticks.this_run());
        }
    }
}
