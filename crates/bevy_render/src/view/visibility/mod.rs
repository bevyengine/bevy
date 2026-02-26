use core::{any::TypeId, mem};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::ReflectComponent,
    query::{Changed, Or, With},
    system::{
        lifetimeless::{Read, SQuery},
        Local, Query, SystemParam,
    },
};
use bevy_log::info_span;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_utils::TypeIdMap;

use crate::{
    sync_world::{MainEntity, MainEntityHashMap, RenderEntity},
    view::RetainedViewEntity,
    Extract,
};

mod range;
use bevy_camera::visibility::*;
pub use range::*;

/// Collection of entities visible from the current view.
///
/// This component is extracted from [`VisibleEntities`].
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct RenderShadowMapVisibleEntities {
    #[reflect(ignore, clone)]
    pub subviews: HashMap<RetainedViewEntity, RenderVisibleEntities>,
}

#[derive(Clone, Component, Default, Debug)]
pub struct RenderVisibleEntities {
    pub classes: TypeIdMap<RenderVisibleEntitiesClass>,
}

/// Stores a list of all entities that are visible from this view, as well as
/// the change lists.
///
/// Note that all lists in this component are guaranteed to be sorted. Thus you
/// can test for the presence of an entity in these lists via binary search.
///
/// Note also that, for 3D meshes, the render-world [`Entity`] values will
/// always be [`Entity::PLACEHOLDER`]. The render-world entities are kept for
/// legacy passes that still need to process visibility of render-world
/// entities.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Debug, Default, Clone)]
pub struct RenderVisibleEntitiesClass {
    /// A sorted list of all entities that don't have [`NoCpuCulling`]
    /// components and are visible from this view.
    #[reflect(ignore, clone)]
    pub entities_cpu_culling: Vec<(Entity, MainEntity)>,
    /// A table of all entities that have [`NoCpuCulling`] components and have
    /// [`bevy_camera::visibility::InheritedVisibility`] set to true.
    pub entities_no_cpu_culling: MainEntityHashMap<Entity>,
    /// A sorted list of all entities that were invisible last frame (including
    /// ones that didn't exist at all last frame) and became visible this frame.
    added_entities: Vec<(Entity, MainEntity)>,
    /// A sorted list of all entities that were visible last frame and became
    /// invisible this frame, including those that were despawned this frame.
    pub removed_entities: Vec<(Entity, MainEntity)>,
}

#[derive(Component, Default)]
pub struct RenderShadowMapVisibleEntitiesCpuCulling {
    pub subviews: HashMap<RetainedViewEntity, RenderVisibleEntitiesCpuCulling>,
}

#[derive(Component, Clone, Default, Debug)]
pub struct RenderVisibleEntitiesCpuCulling {
    pub classes: TypeIdMap<RenderVisibleEntitiesClassCpuCulling>,
}

#[derive(Clone, Default, Debug)]
pub struct RenderVisibleEntitiesClassCpuCulling {
    pub entities: Vec<(Entity, MainEntity)>,
}

/*
#[derive(Resource, Default)]
pub struct RenderVisibleEntitiesGpuCulling {
    pub entities: TypeIdMap<RenderViewEntitiesGpuCulling>,
}

pub struct RenderViewEntitiesGpuCulling {
    pub added_entities: Vec<(Entity, MainEntity)>,
    pub removed_entities: Vec<(Entity, MainEntity)>,
}
*/

impl RenderVisibleEntities {
    pub fn get<QF>(&self) -> Option<&RenderVisibleEntitiesClass>
    where
        QF: 'static,
    {
        self.classes.get(&TypeId::of::<QF>())
    }
}

impl RenderVisibleEntitiesClass {
    fn prepare_for_new_frame(&mut self) {
        self.added_entities.clear();
        self.removed_entities.clear();
    }

    /// Processes a list of visible entities for a new frame, computing the set
    /// of newly-added and newly-removed entities as it goes.
    ///
    /// Entities that participated in CPU culling are in the
    /// `visible_mesh_entities_cpu_culling` list. Entities that opted out of CPU
    /// culling are fetched from the ECS via the
    /// `PreparedVisibilityExtractionSystemParam`.
    fn update_from_cpu(&mut self, visible_mesh_entities_cpu_culling: &[(Entity, MainEntity)]) {
        let _update_from = info_span!("update_from", name = "update_from").entered();

        let old_entities_cpu_culling = mem::take(&mut self.entities_cpu_culling);

        // March over the old and new visible CPU culling entity lists in
        // lockstep, diffing as we go to determine the added and removed
        // entities. The lists must be sorted.
        let mut old_entity_cpu_culling_iter = old_entities_cpu_culling.iter().peekable();
        {
            let _old_entity_cpu_culling_span =
                info_span!("old_entity_cpu_culling", name = "old_entity_cpu_culling").entered();
            for (render_entity, visible_main_entity) in visible_mesh_entities_cpu_culling {
                // Mark entities as removed until we see the one we're looking at.
                while old_entity_cpu_culling_iter
                    .peek()
                    .is_some_and(|(_, main_entity)| *main_entity < *visible_main_entity)
                {
                    self.removed_entities
                        .push(*old_entity_cpu_culling_iter.next().unwrap());
                }

                // Add the visible entity to the list.
                self.entities_cpu_culling
                    .push((*render_entity, *visible_main_entity));

                // If the next entity in the old list isn't equal to the entity we
                // just marked visible, then our entity is newly visible this frame.
                if old_entity_cpu_culling_iter
                    .peek()
                    .is_some_and(|&&(_, main_entity)| main_entity == *visible_main_entity)
                {
                    old_entity_cpu_culling_iter.next();
                } else {
                    self.added_entities
                        .push((*render_entity, *visible_main_entity));
                }
            }
        }

        // Any entities that do CPU culling and that we didn't see yet are
        // removed, so drain them.
        {
            let _old_entity_cpu_culling_removal_span = info_span!(
                "old_entity_cpu_culling_removal",
                name = "old_entity_cpu_culling_removal"
            )
            .entered();
            self.removed_entities
                .extend(old_entity_cpu_culling_iter.copied());
        }
    }

    pub fn add_entity(&mut self, pair: (Entity, MainEntity)) {
        self.added_entities.push(pair);
    }

    pub fn added_entities(&self) -> &[(Entity, MainEntity)] {
        &self.added_entities
    }

    /// Returns true if the given entity pair is known to be visible.
    ///
    /// This checks both the CPU culling visible entries table and the
    /// no-CPU-culling visible entries table.
    pub fn entity_pair_is_visible(&self, entity: Entity, main_entity: MainEntity) -> bool {
        self.entities_cpu_culling
            .binary_search(&(entity, main_entity))
            .is_ok()
            || self
                .entities_no_cpu_culling
                .get(&main_entity)
                .is_some_and(|that_entity| *that_entity == entity)
    }

    /// Iterates over all visible entities.
    ///
    /// This is an expensive operation, so try to avoid doing it unless
    /// necessary (e.g. the view key changed).
    pub fn iter_visible<'a>(&'a self) -> impl Iterator<Item = (&'a Entity, &'a MainEntity)> {
        self.entities_cpu_culling
            .iter()
            .map(|(entity, main_entity)| (entity, main_entity))
            .chain(
                self.entities_no_cpu_culling
                    .iter()
                    .map(|(main_entity, entity)| (entity, main_entity)),
            )
    }

    pub fn sort_added_entities(&mut self) {
        self.added_entities
            .sort_unstable_by_key(|(_, main_entity)| *main_entity);
    }
}

/// A system parameter that goes on any render-world system that needs to
/// extract entities into [`RenderVisibleMeshEntities`].
#[derive(SystemParam)]
pub struct VisibilityExtractionSystemParam<'w, 's> {
    /// Maps entities in the main world to entities in the render world.
    pub mapper: Extract<'w, 's, SQuery<Read<RenderEntity>>>,
}

/// A structure derived from [`VisibilityExtractionSystemParam`] that must be
/// passed to [`RenderVisibleMeshEntities::update_from`].
///
/// This type exists because iterating over [`RemovedComponents`] is destructive
/// and can only be done once. However,
/// [`RenderVisibleMeshEntities::update_from`] needs to do it multiple times.
/// Therefore, we must *prepare* the system parameter, which involves draining
/// the [`RemovedComponents`] list into a vector.
pub struct PreparedVisibilityExtractionSystemParam<'w, 's> {
    /// Maps entities in the main world to entities in the render world.
    pub mapper: Extract<'w, 's, SQuery<Read<RenderEntity>>>,
}

/// The query, part of [`VisibilityExtractionSystemParam`], that searches for
/// entities with [`NoCpuCulling`] that might have changed visibility.
pub type VisibilityExtractionNoCpuCullingChangedQuery = SQuery<
    (Entity, Read<VisibilityClass>, Read<InheritedVisibility>),
    (
        Or<(Changed<NoCpuCulling>, Changed<InheritedVisibility>)>,
        With<NoCpuCulling>,
    ),
>;

pub fn collect_render_visible_entities(
    mut cameras: Query<(
        &mut RenderVisibleEntities,
        Option<&mut RenderVisibleEntitiesCpuCulling>,
    )>,
    mut lights: Query<(
        &MainEntity,
        &mut RenderShadowMapVisibleEntities,
        Option<&mut RenderShadowMapVisibleEntitiesCpuCulling>,
    )>,
    mut visibility_classes: Local<HashSet<TypeId>>,
) {
    // Collect cameras.
    for (mut render_visible_entities, mut maybe_render_visible_entities_cpu_culling) in
        cameras.iter_mut()
    {
        let mut maybe_render_subview_visible_entities_cpu_culling =
            maybe_render_visible_entities_cpu_culling.as_deref_mut();
        collect_render_visible_entities_for_subview(
            &mut render_visible_entities,
            &mut maybe_render_subview_visible_entities_cpu_culling,
            &mut visibility_classes,
        );
    }

    // Collect shadow maps.
    for (
        main_light_entity,
        mut render_shadow_map_visible_entities,
        mut maybe_render_shadow_map_visible_entities_cpu_culling,
    ) in lights.iter_mut()
    {
        for (subview, mut render_visible_entities) in
            render_shadow_map_visible_entities.subviews.iter_mut()
        {
            let mut maybe_render_subview_visible_entities_cpu_culling =
                maybe_render_shadow_map_visible_entities_cpu_culling
                    .as_mut()
                    .and_then(|render_subview_visible_entities_cpu_culling| {
                        render_subview_visible_entities_cpu_culling
                            .subviews
                            .get_mut(subview)
                    });
            collect_render_visible_entities_for_subview(
                render_visible_entities,
                &mut maybe_render_subview_visible_entities_cpu_culling,
                &mut visibility_classes,
            );
        }
    }
}

fn collect_render_visible_entities_for_subview(
    render_visible_entities: &mut RenderVisibleEntities,
    maybe_render_subview_visible_entities_cpu_culling: &mut Option<
        &mut RenderVisibleEntitiesCpuCulling,
    >,
    visibility_classes: &mut HashSet<TypeId>,
) {
    visibility_classes.clear();
    visibility_classes.extend(render_visible_entities.classes.keys().copied());
    if let Some(ref mut render_subview_visible_entities_cpu_culling) =
        *maybe_render_subview_visible_entities_cpu_culling
    {
        visibility_classes.extend(
            render_subview_visible_entities_cpu_culling
                .classes
                .keys()
                .copied(),
        );
    }

    for visibility_class in visibility_classes.iter() {
        let entities = render_visible_entities
            .classes
            .entry(*visibility_class)
            .or_default();

        entities.prepare_for_new_frame();

        let Some(ref mut render_subview_visible_entities_cpu_culling) =
            *maybe_render_subview_visible_entities_cpu_culling
        else {
            continue;
        };
        let Some(render_view_entities_cpu_culling) = render_subview_visible_entities_cpu_culling
            .classes
            .get_mut(visibility_class)
        else {
            continue;
        };
        render_view_entities_cpu_culling
            .entities
            .sort_unstable_by_key(|(_, main_entity)| *main_entity);
        entities.update_from_cpu(&render_view_entities_cpu_culling.entities);
    }
}
