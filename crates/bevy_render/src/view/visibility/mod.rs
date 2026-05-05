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
#[cfg(feature = "trace")]
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

/// Stores a list of all entities that are visible from a single view or
/// subview, as well as the change lists.
///
/// This component is only placed directly on camera entities. Lights instead
/// have a [`RenderShadowMapVisibleEntities`] component that contains one or
/// more [`RenderVisibleEntities`] components, one for each cascade or cubemap
/// side.
///
/// The [`crate::camera::extract_cameras`] and `extract_lights` systems create
/// this object, but they don't populate it. Instead, the
/// [`collect_visible_cpu_culled_entities`] and
/// `collect_gpu_culled_meshes` systems are responsible for
/// updating this component from the lists of entities in
/// [`RenderExtractedVisibleEntities`] and `RenderGpuCulledEntities`,
/// respectively.
#[derive(Clone, Component, Default, Debug)]
pub struct RenderVisibleEntities {
    /// Entities visible from this view or subview, sorted by
    /// [`VisibilityClass`].
    pub classes: TypeIdMap<RenderVisibleEntitiesClass>,
}

/// Collection of entities visible from a single light.
///
/// This component contains one [`RenderVisibleEntities`] object per subview.
/// Directional lights have one subview per cascade, point lights have one
/// subview per cubemap face, and spot lights only have a single subview.
///
/// The `extract_lights` system creates this component, but it doesn't populate
/// it. Instead, the [`collect_visible_cpu_culled_entities`] and
/// `collect_gpu_culled_meshes` systems are responsible for
/// updating this component from the lists of entities in
/// [`RenderExtractedShadowMapVisibleEntities`] and `RenderGpuCulledEntities`,
/// respectively.
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct RenderShadowMapVisibleEntities {
    /// A mapping from each subview (cascade or cubemap face) to the entities
    /// visible from it.
    #[reflect(ignore, clone)]
    pub subviews: HashMap<RetainedViewEntity, RenderVisibleEntities>,
}

/// Stores a list of all entities that are visible from a single view for a
/// single [`VisibilityClass`], as well as the change lists.
///
/// Note that all lists in this component are guaranteed to be sorted. Thus you
/// can test for the presence of an entity in these lists via binary search.
///
/// Note also that, for 3D meshes, the render-world [`Entity`] values will
/// always be [`Entity::PLACEHOLDER`]. The render-world entities are kept for
/// legacy systems that still need to process visibility of render-world
/// entities.
///
/// The [`collect_visible_cpu_culled_entities`] and `collect_gpu_culled_meshes`
/// systems populate this object from the corresponding
/// [`RenderExtractedVisibleEntitiesClass`] object and the
/// `RenderGpuCulledEntities` resource, respectively.
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Debug, Default, Clone)]
pub struct RenderVisibleEntitiesClass {
    /// A sorted list of all entities that don't have [`NoCpuCulling`]
    /// components and are visible from this view.
    #[reflect(ignore, clone)]
    pub entities_cpu_culling: Vec<(Entity, MainEntity)>,

    /// A table of all entities that have [`NoCpuCulling`] components and have
    /// [`bevy_camera::visibility::InheritedVisibility`] set to true.
    ///
    /// The `collect_gpu_culled_meshes` system keeps this up to date.
    pub entities_gpu_culling: MainEntityHashMap<Entity>,

    /// A sorted list of all entities that were invisible last frame (including
    /// ones that didn't exist at all last frame) and became visible this frame.
    added_entities: Vec<(Entity, MainEntity)>,

    /// A sorted list of all entities that were visible last frame and became
    /// invisible this frame, including those that were despawned this frame.
    pub removed_entities: Vec<(Entity, MainEntity)>,
}

/// The entities that the CPU has determined are visible from a single view or
/// subview.
///
/// This component is only placed directly on camera entities. Lights instead
/// have a [`RenderExtractedShadowMapVisibleEntities`] component that contains
/// one or more [`RenderExtractedVisibleEntities`] components, one for each
/// cascade or cubemap side.
///
/// Mesh entities with [`NoCpuCulling`] aren't present in this table. Instead,
/// `collect_gpu_culled_meshes` fetches them directly from the
/// `RenderGpuCulledEntities` list.
///
/// The [`crate::camera::extract_cameras`] and `extract_lights` systems populate
/// this object, and the [`collect_visible_cpu_culled_entities`] system reads it.
#[derive(Component, Clone, Default, Debug)]
pub struct RenderExtractedVisibleEntities {
    /// Entities that the CPU has determined to be visible from this view or
    /// subview, sorted by [`VisibilityClass`].
    pub classes: TypeIdMap<RenderExtractedVisibleEntitiesClass>,
}

/// The entities that the CPU has determined are visible from a single
/// shadow-casting light.
///
/// This component contains one [`RenderExtractedVisibleEntities`] object per
/// subview.  Directional lights have one subview per cascade, point lights have
/// one subview per cubemap face, and spot lights only have a single subview.
///
/// Mesh entities that have [`NoCpuCulling`] components aren't in this list.
/// Instead, `collect_gpu_culled_meshes` fetches them directly
/// from the `RenderGpuCulledEntities` table.
///
/// The `extract_lights` system populates this component, and the
/// [`collect_visible_cpu_culled_entities`] system reads it.
#[derive(Component, Default)]
pub struct RenderExtractedShadowMapVisibleEntities {
    /// A mapping from the subview to the list of entities that the CPU has
    /// determined are visible from it.
    pub subviews: HashMap<RetainedViewEntity, RenderExtractedVisibleEntities>,
}

/// The entities that the CPU has determined are visible from a single view or
/// subview, for a single [`VisibilityClass`].
///
/// Mesh entities that have [`NoCpuCulling`] components aren't in this list.
/// Instead, `collect_gpu_culled_meshes` fetches them directly
/// from the `RenderGpuCulledEntities` table in order to update the
/// [`RenderVisibleEntitiesClass`].
///
/// The [`crate::camera::extract_cameras`] and `extract_lights` systems populate
/// this object, and the [`collect_visible_cpu_culled_entities`] system reads it.
#[derive(Clone, Default, Debug)]
pub struct RenderExtractedVisibleEntitiesClass {
    /// A sorted list of entities that don't have [`NoCpuCulling`] components
    /// and are visible from this view or subview.
    pub entities: Vec<(Entity, MainEntity)>,
}

impl RenderVisibleEntities {
    /// Returns the [`RenderVisibleEntitiesClass`] corresponding to the given
    /// [`VisibilityClass`].
    pub fn get<QF>(&self) -> Option<&RenderVisibleEntitiesClass>
    where
        QF: 'static,
    {
        self.classes.get(&TypeId::of::<QF>())
    }
}

impl RenderVisibleEntitiesClass {
    /// Clears out the lists of added and removed entities in preparation for a
    /// new frame.
    fn prepare_for_new_frame(&mut self) {
        self.added_entities.clear();
        self.removed_entities.clear();
    }

    /// Processes a list of visible entities for a new frame, computing the set
    /// of newly-added and newly-removed entities as it goes.
    ///
    /// This function only handles entities that are culled on CPU (i.e. don't
    /// have `NoCpuCulling` components). Entities that use only GPU culling are
    /// instead fetched from the main world and added to the
    /// `RenderGpuCulledEntities` table.
    fn update_cpu_culled_entities(
        &mut self,
        visible_mesh_entities_cpu_culling: &[(Entity, MainEntity)],
    ) {
        #[cfg(feature = "trace")]
        let _update_from = info_span!("update_from", name = "update_from").entered();

        let old_entities_cpu_culling = mem::take(&mut self.entities_cpu_culling);

        // March over the old and new visible CPU culling entity lists in
        // lockstep, diffing as we go to determine the added and removed
        // entities. The lists must be sorted.
        let mut old_entity_cpu_culling_iter = old_entities_cpu_culling.iter().peekable();
        {
            #[cfg(feature = "trace")]
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
            #[cfg(feature = "trace")]
            let _old_entity_cpu_culling_removal_span = info_span!(
                "old_entity_cpu_culling_removal",
                name = "old_entity_cpu_culling_removal"
            )
            .entered();
            self.removed_entities
                .extend(old_entity_cpu_culling_iter.copied());
        }
    }

    /// Adds a new entity to the [`Self::added_entities`] list.
    ///
    /// After calling this method one or more times, you must call
    /// [`Self::sort_added_entities`] to ensure the [`Self::added_entities`]
    /// list is sorted.
    pub fn add_entity(&mut self, pair: (Entity, MainEntity)) {
        self.added_entities.push(pair);
    }

    /// Returns the list of newly-added entities.
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
                .entities_gpu_culling
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
                self.entities_gpu_culling
                    .iter()
                    .map(|(main_entity, entity)| (entity, main_entity)),
            )
    }

    /// Sorts the [`Self::added_entities`] list.
    ///
    /// You must call this after adding entities to the list via
    /// [`Self::add_entity`].
    pub fn sort_added_entities(&mut self) {
        self.added_entities
            .sort_unstable_by_key(|(_, main_entity)| *main_entity);
    }
}

/// A system parameter that goes on any render-world system that needs to
/// extract entities into [`RenderVisibleEntities`].
#[derive(SystemParam)]
pub struct VisibilityExtractionSystemParam<'w, 's> {
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

/// Updates the [`RenderVisibleEntities`] and [`RenderShadowMapVisibleEntities`]
/// components with the contents of the [`RenderExtractedVisibleEntities`] and
/// the [`RenderExtractedShadowMapVisibleEntities`] components respectively.
///
/// This system only handles CPU-culled entities (i.e. those without
/// [`NoCpuCulling`] components). The `collect_gpu_culled_meshes` system in
/// `bevy_pbr` handles GPU-culled entities.
pub fn collect_visible_cpu_culled_entities(
    mut cameras: Query<(
        &mut RenderVisibleEntities,
        Option<&mut RenderExtractedVisibleEntities>,
    )>,
    mut lights: Query<(
        &mut RenderShadowMapVisibleEntities,
        Option<&mut RenderExtractedShadowMapVisibleEntities>,
    )>,
    mut visibility_classes: Local<HashSet<TypeId>>,
) {
    // Collect cameras.
    for (mut render_visible_entities, mut maybe_render_visible_entities_cpu_culling) in
        cameras.iter_mut()
    {
        let mut maybe_render_subview_visible_entities_cpu_culling =
            maybe_render_visible_entities_cpu_culling.as_deref_mut();
        collect_visible_cpu_culled_entities_for_subview(
            &mut render_visible_entities,
            &mut maybe_render_subview_visible_entities_cpu_culling,
            &mut visibility_classes,
        );
    }

    // Collect shadow maps.
    for (
        mut render_shadow_map_visible_entities,
        mut maybe_render_shadow_map_visible_entities_cpu_culling,
    ) in lights.iter_mut()
    {
        for (subview, render_visible_entities) in
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
            collect_visible_cpu_culled_entities_for_subview(
                render_visible_entities,
                &mut maybe_render_subview_visible_entities_cpu_culling,
                &mut visibility_classes,
            );
        }
    }
}

/// Updates the [`RenderVisibleEntities`] list for a single subview from the
/// applicable [`RenderExtractedVisibleEntities`].
///
/// This only handles CPU-culled entities. The corresponding function for
/// GPU-called entities is `collect_gpu_culled_meshes_for_subview` in
/// `bevy_pbr`.
fn collect_visible_cpu_culled_entities_for_subview(
    render_visible_entities: &mut RenderVisibleEntities,
    maybe_render_subview_visible_entities: &mut Option<&mut RenderExtractedVisibleEntities>,
    visibility_classes: &mut HashSet<TypeId>,
) {
    // Gather up all visibility classes. We need to make sure that the
    // `RenderVisibleEntities` has an entry for each one.
    visibility_classes.clear();
    visibility_classes.extend(render_visible_entities.classes.keys().copied());
    if let Some(ref mut render_subview_visible_entities) = *maybe_render_subview_visible_entities {
        visibility_classes.extend(render_subview_visible_entities.classes.keys().copied());
    }

    // Update the tables of each visibility class.
    for visibility_class in visibility_classes.iter() {
        let entities = render_visible_entities
            .classes
            .entry(*visibility_class)
            .or_default();

        entities.prepare_for_new_frame();

        // Fetch the visibility class's entity table.
        let Some(ref mut render_subview_visible_entities) = *maybe_render_subview_visible_entities
        else {
            continue;
        };
        let Some(render_view_entities) = render_subview_visible_entities
            .classes
            .get_mut(visibility_class)
        else {
            continue;
        };

        // Make sure the entity list is sorted, as this is a requirement for
        // [`RenderVisibleEntitiesClass::update_from_cpu`].
        render_view_entities
            .entities
            .sort_unstable_by_key(|(_, main_entity)| *main_entity);

        entities.update_cpu_culled_entities(&render_view_entities.entities);
    }
}
