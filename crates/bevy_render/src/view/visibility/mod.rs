mod render_layers;

use bevy_math::Vec3A;
pub use render_layers::*;

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_transform::components::GlobalTransform;
use bevy_transform::TransformSystem;
use std::cell::Cell;
use thread_local::ThreadLocal;

use crate::{
    camera::{Camera, CameraProjection, OrthographicProjection, PerspectiveProjection, Projection},
    mesh::Mesh,
    primitives::{Aabb, Frustum, Sphere},
};

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.

/// If an entity is hidden in this way,  all [`Children`] (and all of their children and so on) will also be hidden.
/// This is done by setting the values of their [`ComputedVisibility`] component.
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct Visibility {
    /// Indicates whether this entity is visible. Hidden values will propagate down the entity hierarchy.
    /// If this entity is hidden, all of its descendants will be hidden as well. See [`Children`] and [`Parent`] for
    /// hierarchy info.
    pub is_visible: bool,
}

impl Default for Visibility {
    fn default() -> Self {
        Self { is_visible: true }
    }
}

/// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
#[derive(Component, Clone, Reflect, Debug, Eq, PartialEq, Default)]
#[reflect(Component)]
pub struct ComputedVisibility {
    is_visible_in_hierarchy: bool,
    is_visible_in_view: bool,
}

impl ComputedVisibility {
    /// Whether this entity is visible to something this frame. This is true if and only if [`Self::is_visible_in_hierarchy`] and [`Self::is_visible_in_view`]
    /// are true. This is the canonical method to call to determine if an entity should be drawn.
    /// This value is updated in [`CoreStage::PostUpdate`] during the [`VisibilitySystems::CheckVisibility`] system label. Reading it from the
    /// [`CoreStage::Update`] stage will yield the value from the previous frame.
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.is_visible_in_hierarchy && self.is_visible_in_view
    }

    /// Whether this entity is visible in the entity hierarchy, which is determined by the [`Visibility`] component.
    /// This takes into account "visibility inheritance". If any of this entity's ancestors (see [`Parent`]) are hidden, this entity
    /// will be hidden as well. This value is updated in the [`CoreStage::PostUpdate`] stage in the
    /// [`VisibilitySystems::VisibilityPropagate`] system label.
    #[inline]
    pub fn is_visible_in_hierarchy(&self) -> bool {
        self.is_visible_in_hierarchy
    }

    /// Whether this entity is visible in _any_ view (Cameras, Lights, etc). Each entity type (and view type) should choose how to set this
    /// value. For cameras and drawn entities, this will take into account [`RenderLayers`].
    ///
    /// This value is reset to `false` every frame in [`VisibilitySystems::VisibilityPropagate`] during [`CoreStage::PostUpdate`].
    /// Each entity type then chooses how to set this field in the [`CoreStage::PostUpdate`] stage in the
    /// [`VisibilitySystems::CheckVisibility`] system label. Meshes might use frustum culling to decide if they are visible in a view.
    /// Other entities might just set this to `true` every frame.
    #[inline]
    pub fn is_visible_in_view(&self) -> bool {
        self.is_visible_in_view
    }

    /// Sets `is_visible_in_view` to `true`. This is not reversible for a given frame, as it encodes whether or not this is visible in
    /// _any_ view. This will be automatically reset to `false` every frame in [`VisibilitySystems::VisibilityPropagate`] and then set
    /// to the proper value in [`VisibilitySystems::CheckVisibility`]. This should _only_ be set in systems with the [`VisibilitySystems::CheckVisibility`]
    /// label. Don't call this unless you are defining a custom visibility system. For normal user-defined entity visibility, see [`Visibility`].
    #[inline]
    pub fn set_visible_in_view(&mut self) {
        self.is_visible_in_view = true;
    }
}

/// Use this component to opt-out of built-in frustum culling for Mesh entities
#[derive(Component)]
pub struct NoFrustumCulling;

/// Collection of entities visible from the current view.
///
/// This component contains all entities which are visible from the currently
/// rendered view. The collection is updated automatically by the [`check_visibility()`]
/// system, and renderers can use it to optimize rendering of a particular view, to
/// prevent drawing items not visible from that view.
///
/// This component is intended to be attached to the same entity as the [`Camera`] and
/// the [`Frustum`] defining the view.
///
/// Currently this component is ignored by the sprite renderer, so sprite rendering
/// is not optimized per view.
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component)]
pub struct VisibleEntities {
    #[reflect(ignore)]
    pub entities: Vec<Entity>,
}

impl VisibleEntities {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Entity> {
        self.entities.iter()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum VisibilitySystems {
    CalculateBounds,
    UpdateOrthographicFrusta,
    UpdatePerspectiveFrusta,
    UpdateProjectionFrusta,
    VisibilityPropagate,
    /// Label for the [`check_visibility()`] system updating each frame the [`ComputedVisibility`]
    /// of each entity and the [`VisibleEntities`] of each view.
    CheckVisibility,
}

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use VisibilitySystems::*;

        app.add_system_to_stage(
            CoreStage::PostUpdate,
            calculate_bounds.label(CalculateBounds),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            update_frusta::<OrthographicProjection>
                .label(UpdateOrthographicFrusta)
                .after(TransformSystem::TransformPropagate),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            update_frusta::<PerspectiveProjection>
                .label(UpdatePerspectiveFrusta)
                .after(TransformSystem::TransformPropagate),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            update_frusta::<Projection>
                .label(UpdateProjectionFrusta)
                .after(TransformSystem::TransformPropagate),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            visibility_propagate_system.label(VisibilityPropagate),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            check_visibility
                .label(CheckVisibility)
                .after(CalculateBounds)
                .after(UpdateOrthographicFrusta)
                .after(UpdatePerspectiveFrusta)
                .after(UpdateProjectionFrusta)
                .after(VisibilityPropagate)
                .after(TransformSystem::TransformPropagate),
        );
    }
}

pub fn calculate_bounds(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    without_aabb: Query<(Entity, &Handle<Mesh>), (Without<Aabb>, Without<NoFrustumCulling>)>,
) {
    for (entity, mesh_handle) in &without_aabb {
        if let Some(mesh) = meshes.get(mesh_handle) {
            if let Some(aabb) = mesh.compute_aabb() {
                commands.entity(entity).insert(aabb);
            }
        }
    }
}

pub fn update_frusta<T: Component + CameraProjection + Send + Sync + 'static>(
    mut views: Query<(&GlobalTransform, &T, &mut Frustum)>,
) {
    for (transform, projection, mut frustum) in &mut views {
        let view_projection =
            projection.get_projection_matrix() * transform.compute_matrix().inverse();
        *frustum = Frustum::from_view_projection(
            &view_projection,
            &transform.translation,
            &transform.back(),
            projection.far(),
        );
    }
}

fn visibility_propagate_system(
    mut root_query: Query<
        (
            Option<&Children>,
            &Visibility,
            &mut ComputedVisibility,
            Entity,
        ),
        Without<Parent>,
    >,
    mut visibility_query: Query<(&Visibility, &mut ComputedVisibility, &Parent)>,
    children_query: Query<&Children, (With<Parent>, With<Visibility>, With<ComputedVisibility>)>,
) {
    for (children, visibility, mut computed_visibility, entity) in root_query.iter_mut() {
        computed_visibility.is_visible_in_hierarchy = visibility.is_visible;
        // reset "view" visibility here ... if this entity should be drawn a future system should set this to true
        computed_visibility.is_visible_in_view = false;
        if let Some(children) = children {
            for child in children.iter() {
                let _ = propagate_recursive(
                    computed_visibility.is_visible_in_hierarchy,
                    &mut visibility_query,
                    &children_query,
                    *child,
                    entity,
                );
            }
        }
    }
}

fn propagate_recursive(
    parent_visible: bool,
    visibility_query: &mut Query<(&Visibility, &mut ComputedVisibility, &Parent)>,
    children_query: &Query<&Children, (With<Parent>, With<Visibility>, With<ComputedVisibility>)>,
    entity: Entity,
    expected_parent: Entity,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    let is_visible = {
        let (visibility, mut computed_visibility, child_parent) =
            visibility_query.get_mut(entity).map_err(drop)?;
        assert_eq!(
            child_parent.get(), expected_parent,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        computed_visibility.is_visible_in_hierarchy = visibility.is_visible && parent_visible;
        // reset "view" visibility here ... if this entity should be drawn a future system should set this to true
        computed_visibility.is_visible_in_view = false;
        computed_visibility.is_visible_in_hierarchy
    };

    for child in children_query.get(entity).map_err(drop)?.iter() {
        let _ = propagate_recursive(is_visible, visibility_query, children_query, *child, entity);
    }
    Ok(())
}

// the batch size used for check_visibility, chosen because this number tends to perform well
const VISIBLE_ENTITIES_QUERY_BATCH_SIZE: usize = 1024;

/// System updating the visibility of entities each frame.
///
/// The system is labelled with [`VisibilitySystems::CheckVisibility`]. Each frame, it updates the
/// [`ComputedVisibility`] of all entities, and for each view also compute the [`VisibleEntities`]
/// for that view.
pub fn check_visibility(
    mut thread_queues: Local<ThreadLocal<Cell<Vec<Entity>>>>,
    mut view_query: Query<(&mut VisibleEntities, &Frustum, Option<&RenderLayers>), With<Camera>>,
    mut visible_aabb_query: Query<(
        Entity,
        &mut ComputedVisibility,
        Option<&RenderLayers>,
        &Aabb,
        &GlobalTransform,
        Option<&NoFrustumCulling>,
    )>,
    mut visible_no_aabb_query: Query<
        (Entity, &mut ComputedVisibility, Option<&RenderLayers>),
        Without<Aabb>,
    >,
) {
    for (mut visible_entities, frustum, maybe_view_mask) in &mut view_query {
        let view_mask = maybe_view_mask.copied().unwrap_or_default();
        visible_entities.entities.clear();
        visible_aabb_query.par_for_each_mut(
            VISIBLE_ENTITIES_QUERY_BATCH_SIZE,
            |(
                entity,
                mut computed_visibility,
                maybe_entity_mask,
                model_aabb,
                transform,
                maybe_no_frustum_culling,
            )| {
                // skip computing visibility for entities that are configured to be hidden. is_visible_in_view has already been set to false
                // in visibility_propagate_system
                if !computed_visibility.is_visible_in_hierarchy() {
                    return;
                }

                let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
                if !view_mask.intersects(&entity_mask) {
                    return;
                }

                // If we have an aabb and transform, do frustum culling
                if maybe_no_frustum_culling.is_none() {
                    let model = transform.compute_matrix();
                    let model_sphere = Sphere {
                        center: model.transform_point3a(model_aabb.center),
                        radius: (Vec3A::from(transform.scale) * model_aabb.half_extents).length(),
                    };
                    // Do quick sphere-based frustum culling
                    if !frustum.intersects_sphere(&model_sphere, false) {
                        return;
                    }
                    // If we have an aabb, do aabb-based frustum culling
                    if !frustum.intersects_obb(model_aabb, &model, false) {
                        return;
                    }
                }

                computed_visibility.is_visible_in_view = true;
                let cell = thread_queues.get_or_default();
                let mut queue = cell.take();
                queue.push(entity);
                cell.set(queue);
            },
        );

        visible_no_aabb_query.par_for_each_mut(
            VISIBLE_ENTITIES_QUERY_BATCH_SIZE,
            |(entity, mut computed_visibility, maybe_entity_mask)| {
                // skip computing visibility for entities that are configured to be hidden. is_visible_in_view has already been set to false
                // in visibility_propagate_system
                if !computed_visibility.is_visible_in_hierarchy() {
                    return;
                }

                let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
                if !view_mask.intersects(&entity_mask) {
                    return;
                }

                computed_visibility.is_visible_in_view = true;
                let cell = thread_queues.get_or_default();
                let mut queue = cell.take();
                queue.push(entity);
                cell.set(queue);
            },
        );

        for cell in thread_queues.iter_mut() {
            visible_entities.entities.append(cell.get_mut());
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_app::prelude::*;
    use bevy_ecs::prelude::*;

    use super::*;

    use bevy_hierarchy::BuildWorldChildren;

    #[test]
    fn visibility_propagation() {
        let mut app = App::new();
        app.add_system(visibility_propagate_system);

        let root1 = app
            .world
            .spawn()
            .insert_bundle((
                Visibility { is_visible: false },
                ComputedVisibility::default(),
            ))
            .id();
        let root1_child1 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root1_child2 = app
            .world
            .spawn()
            .insert_bundle((
                Visibility { is_visible: false },
                ComputedVisibility::default(),
            ))
            .id();
        let root1_child1_grandchild1 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root1_child2_grandchild1 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();

        app.world
            .entity_mut(root1)
            .push_children(&[root1_child1, root1_child2]);
        app.world
            .entity_mut(root1_child1)
            .push_children(&[root1_child1_grandchild1]);
        app.world
            .entity_mut(root1_child2)
            .push_children(&[root1_child2_grandchild1]);

        let root2 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root2_child1 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root2_child2 = app
            .world
            .spawn()
            .insert_bundle((
                Visibility { is_visible: false },
                ComputedVisibility::default(),
            ))
            .id();
        let root2_child1_grandchild1 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root2_child2_grandchild1 = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), ComputedVisibility::default()))
            .id();

        app.world
            .entity_mut(root2)
            .push_children(&[root2_child1, root2_child2]);
        app.world
            .entity_mut(root2_child1)
            .push_children(&[root2_child1_grandchild1]);
        app.world
            .entity_mut(root2_child2)
            .push_children(&[root2_child2_grandchild1]);

        app.update();

        let is_visible = |e: Entity| {
            app.world
                .entity(e)
                .get::<ComputedVisibility>()
                .unwrap()
                .is_visible_in_hierarchy
        };
        assert!(
            !is_visible(root1),
            "invisibility propagates down tree from root"
        );
        assert!(
            !is_visible(root1_child1),
            "invisibility propagates down tree from root"
        );
        assert!(
            !is_visible(root1_child2),
            "invisibility propagates down tree from root"
        );
        assert!(
            !is_visible(root1_child1_grandchild1),
            "invisibility propagates down tree from root"
        );
        assert!(
            !is_visible(root1_child2_grandchild1),
            "invisibility propagates down tree from root"
        );

        assert!(
            is_visible(root2),
            "visibility propagates down tree from root"
        );
        assert!(
            is_visible(root2_child1),
            "visibility propagates down tree from root"
        );
        assert!(
            !is_visible(root2_child2),
            "visibility propagates down tree from root, but local invisibility is preserved"
        );
        assert!(
            is_visible(root2_child1_grandchild1),
            "visibility propagates down tree from root"
        );
        assert!(
            !is_visible(root2_child2_grandchild1),
            "child's invisibility propagates down to grandchild"
        );
    }
}
