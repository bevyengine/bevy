use core::{any::TypeId, mem};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    lifecycle::RemovedComponents,
    prelude::ReflectComponent,
    query::{Changed, Or},
    system::{
        lifetimeless::{Read, SQuery},
        SystemParam,
    },
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_utils::TypeIdMap;

use crate::{
    sync_world::{MainEntity, MainEntityHashMap, RenderEntity},
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
pub struct RenderVisibleEntities {
    #[reflect(ignore, clone)]
    pub entities: TypeIdMap<RenderVisibleMeshEntities>,
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
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct RenderVisibleMeshEntities {
    /// A sorted list of all entities that don't have [`NoCpuCulling`]
    /// components and are visible from this view.
    #[reflect(ignore, clone)]
    pub entities_cpu_culling: Vec<(Entity, MainEntity)>,
    /// A table of all entities that have [`NoCpuCulling`] components and have
    /// [`bevy_camera::visibility::InheritedVisibility`] set to true.
    pub entities_no_cpu_culling: MainEntityHashMap<Entity>,
    /// A sorted list of all entities that were invisible last frame (including
    /// ones that didn't exist at all last frame) and became visible this frame.
    pub added_entities: Vec<(Entity, MainEntity)>,
    /// A sorted list of all entities that were visible last frame and became
    /// invisible this frame, including those that were despawned this frame.
    pub removed_entities: Vec<(Entity, MainEntity)>,
}

impl RenderVisibleEntities {
    pub fn get<QF>(&self) -> Option<&RenderVisibleMeshEntities>
    where
        QF: 'static,
    {
        self.entities.get(&TypeId::of::<QF>())
    }
}

impl RenderVisibleMeshEntities {
    /// Processes a list of visible entities for a new frame, computing the set
    /// of newly-added and newly-removed entities as it goes.
    ///
    /// Entities that participated in CPU culling are in the
    /// `visible_mesh_entities_cpu_culling` list. Entities that opted out of CPU
    /// culling are fetched from the ECS via the
    /// `PreparedVisibilityExtractionSystemParam`.
    pub fn update_from(
        &mut self,
        visibility_extraction_system_param: &PreparedVisibilityExtractionSystemParam,
        visible_mesh_entities_cpu_culling: &[Entity],
        visibility_class: TypeId,
    ) {
        let PreparedVisibilityExtractionSystemParam {
            mapper,
            no_cpu_culling_changed_entities,
            no_cpu_culling_removed_entities,
        } = visibility_extraction_system_param;

        let old_entities_cpu_culling = mem::take(&mut self.entities_cpu_culling);
        self.added_entities.clear();
        self.removed_entities.clear();

        // March over the old and new visible CPU culling entity lists in
        // lockstep, diffing as we go to determine the added and removed
        // entities. The lists must be sorted.
        let mut old_entity_cpu_culling_iter = old_entities_cpu_culling.iter().peekable();
        for &visible_main_entity in visible_mesh_entities_cpu_culling {
            let visible_main_entity = MainEntity::from(visible_main_entity);

            // Mark entities as removed until we see the one we're looking at.
            while old_entity_cpu_culling_iter
                .peek()
                .is_some_and(|(_, main_entity)| *main_entity < visible_main_entity)
            {
                self.removed_entities
                    .push(*old_entity_cpu_culling_iter.next().unwrap());
            }

            // Add the visible entity to the list.
            let render_entity = mapper
                .get(*visible_main_entity)
                .cloned()
                .unwrap_or(RenderEntity::from(Entity::PLACEHOLDER));
            self.entities_cpu_culling
                .push((*render_entity, visible_main_entity));

            // If the next entity in the old list isn't equal to the entity we
            // just marked visible, then our entity is newly visible this frame.
            if old_entity_cpu_culling_iter
                .peek()
                .is_some_and(|&&(_, main_entity)| main_entity == visible_main_entity)
            {
                old_entity_cpu_culling_iter.next();
            } else {
                self.added_entities
                    .push((*render_entity, visible_main_entity));
            }
        }

        // Any entities that do CPU culling and that we didn't see yet are
        // removed, so drain them.
        self.removed_entities
            .extend(old_entity_cpu_culling_iter.copied());

        // Now process all changed entities that have `NoCpuCulling`.
        for (visible_main_entity, entity_visibility_class, inherited_visibility) in
            no_cpu_culling_changed_entities.iter()
        {
            let visible_main_entity = MainEntity::from(visible_main_entity);
            let render_entity = mapper
                .get(*visible_main_entity)
                .cloned()
                .unwrap_or(RenderEntity::from(Entity::PLACEHOLDER));

            // If the entity is invisible, then remove it from the set of
            // visible entities if it's there.
            if !entity_visibility_class.contains(&visibility_class) || !inherited_visibility.get() {
                if self
                    .entities_no_cpu_culling
                    .remove(&visible_main_entity)
                    .is_some()
                {
                    self.removed_entities
                        .push((*render_entity, visible_main_entity));
                }
                continue;
            }

            // Otherwise, it's a newly-visible entity (as far as the CPU is
            // concerned). Mark it as such.
            self.added_entities
                .push((*render_entity, visible_main_entity));
            self.entities_no_cpu_culling
                .insert(visible_main_entity, *render_entity);
        }

        // Our list of added entities must be sorted. Ensure that.
        self.added_entities
            .sort_unstable_by_key(|(_, main_entity)| *main_entity);

        // Finally, remove entities that had [`NoCpuCulling`] removed.
        for removed_main_entity in no_cpu_culling_removed_entities {
            let removed_main_entity = MainEntity::from(*removed_main_entity);

            // The fact that an entity is present in
            // `RemovedComponents<NoCpuCulling>` doesn't necessarily mean that
            // it's now invisible. First, the entity might simply have had
            // [`NoCpuCulling`] removed in order to go from being culled on GPU
            // to culled on CPU; in that case, the entity should remain visible.
            // We check for that situation:
            if self
                .entities_cpu_culling
                .binary_search_by_key(&removed_main_entity, |(_, main_entity)| *main_entity)
                .is_ok()
            {
                continue;
            }

            // Second, the entity might have had [`NoCpuCulling`] removed and
            // then re-added in the same frame, in which case, again, it should
            // remain visible. We likewise check for that situation:
            if self
                .added_entities
                .binary_search_by_key(&removed_main_entity, |(_, main_entity)| *main_entity)
                .is_ok()
            {
                continue;
            }

            // If we got here, the entity is now known to be invisible. Remove
            // it.
            self.removed_entities
                .push((Entity::PLACEHOLDER, removed_main_entity));
            self.entities_no_cpu_culling.remove(&removed_main_entity);
        }
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
}

/// A system parameter that goes on any render-world system that needs to
/// extract entities into [`RenderVisibleMeshEntities`].
#[derive(SystemParam)]
pub struct VisibilityExtractionSystemParam<'w, 's> {
    /// Maps entities in the main world to entities in the render world.
    pub mapper: Extract<'w, 's, SQuery<Read<RenderEntity>>>,
    /// Entities that have [`NoCpuCulling`] components and have changed in such
    /// a way as to affect their CPU-side visibility.
    pub no_cpu_culling_added_entities:
        Extract<'w, 's, VisibilityExtractionNoCpuCullingChangedQuery>,
    /// Entities that have had their [`NoCpuCulling`] components removed.
    pub no_cpu_culling_removed_entities:
        Extract<'w, 's, RemovedComponents<'static, 'static, NoCpuCulling>>,
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
    /// Entities that have [`NoCpuCulling`] components and have changed in such
    /// a way as to affect their CPU-side visibility.
    pub no_cpu_culling_changed_entities:
        Extract<'w, 's, VisibilityExtractionNoCpuCullingChangedQuery>,
    /// Entities that have had their [`NoCpuCulling`] components removed.
    ///
    /// This is the same as
    /// [`VisibilityExtractionSystemParam::no_cpu_culling_removed_entities`],
    /// but collected into a vector.
    pub no_cpu_culling_removed_entities: Vec<Entity>,
}

/// The query, part of [`VisibilityExtractionSystemParam`], that searches for
/// entities with [`NoCpuCulling`] that might have changed visibility.
pub type VisibilityExtractionNoCpuCullingChangedQuery = SQuery<
    (Entity, Read<VisibilityClass>, Read<InheritedVisibility>),
    Or<(Changed<NoCpuCulling>, Changed<InheritedVisibility>)>,
>;

impl<'w, 's> VisibilityExtractionSystemParam<'w, 's> {
    /// Converts a [`VisibilityExtractionSystemParam`] to a
    /// [`PreparedVisibilityExtractionSystemParam`].
    ///
    /// This simply drains the
    /// [`VisibilityExtractionSystemParam::no_cpu_culling_removed_entities`]
    /// list into a vector.
    pub fn prepare(mut self) -> PreparedVisibilityExtractionSystemParam<'w, 's> {
        let no_cpu_culling_removed_entities = self.no_cpu_culling_removed_entities.read().collect();
        PreparedVisibilityExtractionSystemParam {
            mapper: self.mapper,
            no_cpu_culling_changed_entities: self.no_cpu_culling_added_entities,
            no_cpu_culling_removed_entities,
        }
    }
}
