use crate::extract_resource::ExtractResource;
use crate::sync_world::{MainEntity, MainEntityHashSet};
use crate::view::VisibilitySystems::CheckVisibility;
use crate::view::VisibleEntities;
use crate::RenderApp;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::{Component, Tick};
use bevy_ecs::entity::hash_map::EntityHashMap;
use bevy_ecs::entity::hash_set::EntityHashSet;
use bevy_ecs::entity::{Entity, EntityHash};
use bevy_ecs::prelude::SystemSet;
use bevy_ecs::query::{QueryFilter, QueryItem, ReadOnlyQueryData};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::schedule::IntoSystemSetConfigs;
use bevy_ecs::system::{Commands, Local, Query, Res, ResMut, SystemChangeTick};
use bevy_platform_support::collections::{HashMap, HashSet};
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_utils::{Parallel, TypeIdMap};
use core::marker::PhantomData;
use crossbeam_channel::{Receiver, Sender};
use std::any::TypeId;

pub struct SpecializationPlugin;

impl Plugin for SpecializationPlugin {
    fn build(&self, app: &mut App) {
        use SpecializationSystems::*;

        app.configure_sets(
            PostUpdate,
            (
                UpdateLastSpecialized.after(CheckVisibility),
                CheckSpecialization.after(UpdateLastSpecialized),
                MarkEntitiesToSpecialize.after(CheckSpecialization),
            ),
        )
        .register_type::<EntitySpecializationTicks>()
        .init_resource::<EntitySpecializationTicks>()
        .register_type::<LastSpecializedTicks>()
        .init_resource::<LastSpecializedTicks>();
    }
}

pub struct CheckSpecializationPlugin<M>(PhantomData<M>);

impl<M> Default for CheckSpecializationPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M> Plugin for CheckSpecializationPlugin<M>
where
    M: CheckSpecialization,
{
    fn build(&self, app: &mut App) {
        use SpecializationSystems::*;
        app.init_resource::<ViewKeyCache<M>>().add_systems(
            PostUpdate,
            (
                update_last_specialized::<M>.in_set(UpdateLastSpecialized),
                (
                    check_entities_needs_specialization::<M>,
                    check_views_need_specialization::<M>,
                )
                    .in_set(CheckSpecialization),
                mark_entities_for_specialization::<M>.in_set(MarkEntitiesToSpecialize),
            ),
        );
    }

    fn finish(&self, app: &mut App) {
        let (tx, rx) = crossbeam_channel::unbounded();
        app.insert_resource(EntitySpecializedReceiver::<M>::new(rx));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(EntitySpecializedSender::<M>::new(tx));
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum SpecializationSystems {
    UpdateLastSpecialized,
    CheckSpecialization,
    MarkEntitiesToSpecialize,
}

#[derive(Clone, Resource, Default, Debug, Reflect)]
#[reflect(Default)]
pub struct EntitySpecializationTicks {
    #[reflect(ignore)]
    pub entities: TypeIdMap<EntityHashMap<Tick>>,
}

#[derive(Clone, Resource, Default, Debug, Reflect)]
#[reflect(Default)]
pub struct LastSpecializedTicks {
    // (entity, view_entity) -> tick
    #[reflect(ignore)]
    pub entity_views: TypeIdMap<HashMap<(Entity, Entity), Tick, EntityHash>>,
}

#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Default)]
pub struct EntitiesToSpecialize {
    #[reflect(ignore)]
    pub entities: TypeIdMap<Vec<Entity>>,
}

#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Default)]
pub struct RenderEntitiesToSpecialize {
    #[reflect(ignore)]
    pub entities: TypeIdMap<Vec<MainEntity>>,
}

#[derive(Resource)]
pub struct EntitySpecializedSender<M> {
    tx: Sender<(Entity, Entity)>,
    _marker: PhantomData<M>,
}

impl<M> EntitySpecializedSender<M> {
    pub fn new(tx: Sender<(Entity, Entity)>) -> Self {
        Self {
            tx,
            _marker: Default::default(),
        }
    }

    pub fn send(&self, entity: Entity, view_entity: Entity) {
        self.tx.send((entity, view_entity)).unwrap();
    }
}

#[derive(Resource)]
pub struct EntitySpecializedReceiver<M> {
    rx: Receiver<(Entity, Entity)>,
    _marker: PhantomData<M>,
}

impl<M> EntitySpecializedReceiver<M> {
    pub fn new(rx: Receiver<(Entity, Entity)>) -> Self {
        Self {
            rx,
            _marker: Default::default(),
        }
    }

    pub fn recv(&self) -> Option<(Entity, Entity)> {
        self.rx.try_recv().ok()
    }
}

pub fn update_last_specialized<M>(
    mut last_specialized_ticks: ResMut<LastSpecializedTicks>,
    entity_specialized_receiver: Res<EntitySpecializedReceiver<M>>,
    ticks: SystemChangeTick,
) where
    M: CheckSpecialization,
{
    while let Some((entity, view_entity)) = entity_specialized_receiver.recv() {
        let last_specialized_ticks = last_specialized_ticks
            .entity_views
            .entry(TypeId::of::<M::VisibilityClass>())
            .or_default();
        last_specialized_ticks.insert((entity, view_entity), ticks.this_run());
    }
}

/// Marks entities that need specialization in the render world.
pub fn mark_entities_for_specialization<M>(
    entity_specialization_ticks: Res<EntitySpecializationTicks>,
    last_specialized_ticks: Res<LastSpecializedTicks>,
    mut views: Query<(Entity, &VisibleEntities, &mut EntitiesToSpecialize)>,
    ticks: SystemChangeTick,
) where
    M: CheckSpecialization,
{
    for (view_entity, visible_entities, mut entities_to_specialize) in views.iter_mut() {
        let entities_to_specialize = entities_to_specialize
            .entities
            .entry(TypeId::of::<M::VisibilityClass>())
            .or_default();
        entities_to_specialize.clear();

        for entity in visible_entities.iter(TypeId::of::<M::VisibilityClass>()) {
            let entity_specialization_ticks = entity_specialization_ticks
                .entities
                .get(&TypeId::of::<M::VisibilityClass>())
                .unwrap();
            let last_specialized_ticks = last_specialized_ticks
                .entity_views
                .get(&TypeId::of::<M::VisibilityClass>());
            let last_specialized_tick =
                last_specialized_ticks.and_then(|ticks| ticks.get(&(*entity, view_entity)));
            if last_specialized_tick.is_none()
                || entity_specialization_ticks
                    .get(entity)
                    .unwrap()
                    .is_newer_than(*last_specialized_tick.unwrap(), ticks.this_run())
                || entity_specialization_ticks
                    .get(&view_entity)
                    .unwrap()
                    .is_newer_than(*last_specialized_tick.unwrap(), ticks.this_run())
            {
                println!("Marking entity for specialization: {:?}", entity);
                entities_to_specialize.push(*entity);
            }
        }
    }
}

pub fn check_entities_needs_specialization<M>(
    mut thread_queues: Local<Parallel<Vec<Entity>>>,
    mut needs_specialization: Query<(Entity,), M::EntityQueryFilter>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks>,
    ticks: SystemChangeTick,
) where
    M: CheckSpecialization,
{
    needs_specialization.par_iter_mut().for_each_init(
        || thread_queues.borrow_local_mut(),
        |queue, query_item| {
            let (entity,) = query_item;
            queue.push(entity);
        },
    );

    let mut entity_specialization_ticks = entity_specialization_ticks
        .entities
        .entry(TypeId::of::<M::VisibilityClass>())
        .or_default();
    let size = thread_queues.iter_mut().map(|queue| queue.len()).sum();
    entity_specialization_ticks.reserve(size);
    let this_run = ticks.this_run();
    for queue in thread_queues.iter_mut() {
        for entity in queue.drain(..) {
            // Update the entity's specialization tick with this run's tick
            entity_specialization_ticks.insert(entity, this_run);
        }
    }
}

#[derive(Resource, Deref, DerefMut, ExtractResource, Clone)]
pub struct ViewKeyCache<M>(EntityHashMap<M::ViewKey>)
where
    M: CheckSpecialization;

impl<M> Default for ViewKeyCache<M>
where
    M: CheckSpecialization,
{
    fn default() -> Self {
        Self(EntityHashMap::default())
    }
}

pub trait CheckSpecialization: Component {
    type ViewKey: PartialEq + Send + Sync + 'static;
    type VisibilityClass: Component;
    type ViewKeyQueryData: ReadOnlyQueryData + 'static;
    type EntityQueryFilter: QueryFilter + 'static;

    fn get_view_key<'w>(view_query: QueryItem<'w, Self::ViewKeyQueryData>) -> Self::ViewKey;
}

pub fn check_views_need_specialization<M>(
    mut view_key_cache: ResMut<ViewKeyCache<M>>,
    mut view_specialization_ticks: ResMut<EntitySpecializationTicks>,
    mut views: Query<(Entity, M::ViewKeyQueryData)>,
    ticks: SystemChangeTick,
) where
    M: CheckSpecialization,
{
    let view_specialization_ticks = view_specialization_ticks
        .entities
        .entry(TypeId::of::<M::VisibilityClass>())
        .or_default();
    for (view_entity, view_query) in views.iter_mut() {
        let view_key = M::get_view_key(view_query);
        if let Some(current_key) = view_key_cache.get_mut(&view_entity) {
            if *current_key != view_key {
                view_key_cache.insert(view_entity, view_key);
                view_specialization_ticks.insert(view_entity, ticks.this_run());
            }
        } else {
            view_key_cache.insert(view_entity, view_key);
            view_specialization_ticks.insert(view_entity, ticks.this_run());
        }
    }
}
