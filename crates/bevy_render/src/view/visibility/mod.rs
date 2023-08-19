mod render_layers;

use bevy_derive::{Deref, DerefMut};
pub use render_layers::*;

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystem};
use std::cell::Cell;
use thread_local::ThreadLocal;

use crate::{
    camera::{
        camera_system, Camera, CameraProjection, OrthographicProjection, PerspectiveProjection,
        Projection,
    },
    mesh::Mesh,
    primitives::{Aabb, Frustum, Sphere},
};

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.
///
/// If an entity is hidden in this way, all [`Children`] (and all of their children and so on) who
/// are set to [`Inherited`](Self::Inherited) will also be hidden.
///
/// This is done by the `visibility_propagate_system` which uses the entity hierarchy and
/// `Visibility` to set the values of each entity's [`ComputedVisibility`] component.
#[derive(Component, Clone, Copy, Reflect, Debug, PartialEq, Eq, Default)]
#[reflect(Component, Default)]
pub enum Visibility {
    /// An entity with `Visibility::Inherited` will inherit the Visibility of its [`Parent`].
    ///
    /// A root-level entity that is set to `Inherited` will be visible.
    #[default]
    Inherited,
    /// An entity with `Visibility::Hidden` will be unconditionally hidden.
    Hidden,
    /// An entity with `Visibility::Visible` will be unconditionally visible.
    ///
    /// Note that an entity with `Visibility::Visible` will be visible regardless of whether the
    /// [`Parent`] entity is hidden.
    Visible,
}

// Allows `&Visibility == Visibility`
impl std::cmp::PartialEq<Visibility> for &Visibility {
    #[inline]
    fn eq(&self, other: &Visibility) -> bool {
        **self == *other
    }
}

// Allows `Visibility == &Visibility`
impl std::cmp::PartialEq<&Visibility> for Visibility {
    #[inline]
    fn eq(&self, other: &&Visibility) -> bool {
        *self == **other
    }
}

/// Whether or not an entity is visible in the hierarchy.
/// This will not be accurate until [`VisibilityPropagate`] runs in the [`PostUpdate`] schedule.
///
/// If this is false, then [`VisibleInView`] should also be false.
///
/// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
#[derive(Component, Deref, DerefMut, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
pub struct VisibleInHierarchy(bool);

impl VisibleInHierarchy {
    pub const HIDDEN: Self = Self(false);

    /// Returns `true` if the entity is visible in the hierarchy.
    /// Otherwise, returns `false`.
    pub fn get(self) -> bool {
        self.0
    }
}

/// Algorithmically-computed indication or whether an entity is visible and should be extracted for rendering.
///
/// This will be reset to `false` at the beginning of [`PostUpdate`], and then set to true by
/// [`CheckVisibility`] systems. Because of this, values of this type will be marked as changed on
/// nearly every frame, even when they do not change.
///
/// [`VisibilitySystems::CheckVisibility`]
#[derive(Component, Deref, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
pub struct VisibleInView(bool);

impl VisibleInView {
    pub const HIDDEN: Self = Self(false);

    /// Returns `true` if the entity is visible in any view.
    /// Otherwise, returns `false`.
    pub fn get(self) -> bool {
        self.0
    }

    pub fn set(&mut self) {
        self.0 = true;
    }
}

/// A [`Bundle`] of the [`Visibility`] and [`ComputedVisibility`]
/// [`Component`](bevy_ecs::component::Component)s, which describe the visibility of an entity.
///
/// * To show or hide an entity, you should set its [`Visibility`].
/// * To get the computed visibility of an entity, you should get its [`ComputedVisibility`].
/// * For visibility hierarchies to work correctly, you must have both a [`Visibility`] and a [`ComputedVisibility`].
///   * You may use the [`VisibilityBundle`] to guarantee this.
#[derive(Bundle, Debug, Default)]
pub struct VisibilityBundle {
    /// The visibility of the entity.
    pub visibility: Visibility,
    // The inherited visibility of the entity.
    pub visible_in_hierarchy: VisibleInHierarchy,
    // The computed visibility of the entity.
    pub visible_in_view: VisibleInView,
}

/// Use this component to opt-out of built-in frustum culling for Mesh entities
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum VisibilitySystems {
    CalculateBounds,
    CalculateBoundsFlush,
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

        app
            // We add an AABB component in CalculateBounds, which must be ready on the same frame.
            .add_systems(PostUpdate, apply_deferred.in_set(CalculateBoundsFlush))
            .configure_set(PostUpdate, CalculateBoundsFlush.after(CalculateBounds))
            .add_systems(
                PostUpdate,
                (
                    calculate_bounds.in_set(CalculateBounds),
                    update_frusta::<OrthographicProjection>
                        .in_set(UpdateOrthographicFrusta)
                        .after(camera_system::<OrthographicProjection>)
                        .after(TransformSystem::TransformPropagate)
                        // We assume that no camera will have more than one projection component,
                        // so these systems will run independently of one another.
                        // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                        .ambiguous_with(update_frusta::<PerspectiveProjection>)
                        .ambiguous_with(update_frusta::<Projection>),
                    update_frusta::<PerspectiveProjection>
                        .in_set(UpdatePerspectiveFrusta)
                        .after(camera_system::<PerspectiveProjection>)
                        .after(TransformSystem::TransformPropagate)
                        // We assume that no camera will have more than one projection component,
                        // so these systems will run independently of one another.
                        // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                        .ambiguous_with(update_frusta::<Projection>),
                    update_frusta::<Projection>
                        .in_set(UpdateProjectionFrusta)
                        .after(camera_system::<Projection>)
                        .after(TransformSystem::TransformPropagate),
                    visibility_propagate_system.in_set(VisibilityPropagate),
                    check_visibility
                        .in_set(CheckVisibility)
                        .after(CalculateBoundsFlush)
                        .after(UpdateOrthographicFrusta)
                        .after(UpdatePerspectiveFrusta)
                        .after(UpdateProjectionFrusta)
                        .after(VisibilityPropagate)
                        .after(TransformSystem::TransformPropagate),
                ),
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
    mut views: Query<
        (&GlobalTransform, &T, &mut Frustum),
        Or<(Changed<GlobalTransform>, Changed<T>)>,
    >,
) {
    for (transform, projection, mut frustum) in &mut views {
        let view_projection =
            projection.get_projection_matrix() * transform.compute_matrix().inverse();
        *frustum = Frustum::from_view_projection_custom_far(
            &view_projection,
            &transform.translation(),
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
            &mut VisibleInHierarchy,
            &mut VisibleInView,
            Entity,
        ),
        Without<Parent>,
    >,
    mut visibility_query: Query<(
        &Visibility,
        &mut VisibleInHierarchy,
        &mut VisibleInView,
        &Parent,
    )>,
    children_query: Query<
        &Children,
        (
            With<Parent>,
            With<Visibility>,
            (With<VisibleInHierarchy>, With<VisibleInView>),
        ),
    >,
) {
    for (children, visibility, mut visible_in_hierarchy, mut visible_in_view, entity) in
        root_query.iter_mut()
    {
        let is_visible = matches!(visibility, Visibility::Inherited | Visibility::Visible);
        visible_in_hierarchy.set_if_neq(VisibleInHierarchy(is_visible));
        // reset "view" visibility here ... if this entity should be drawn a future system should set this to true
        visible_in_view.set_if_neq(VisibleInView(false));
        if let Some(children) = children {
            for child in children.iter() {
                let _ = propagate_recursive(
                    is_visible,
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
    visibility_query: &mut Query<(
        &Visibility,
        &mut VisibleInHierarchy,
        &mut VisibleInView,
        &Parent,
    )>,
    children_query: &Query<
        &Children,
        (
            With<Parent>,
            With<Visibility>,
            (With<VisibleInHierarchy>, With<VisibleInView>),
        ),
    >,
    entity: Entity,
    expected_parent: Entity,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    let is_visible = {
        let (visibility, mut visible_in_hierarchy, mut visible_in_view, child_parent) =
            visibility_query.get_mut(entity).map_err(drop)?;
        assert_eq!(
            child_parent.get(), expected_parent,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        let is_visible = (parent_visible && visibility == Visibility::Inherited)
            || visibility == Visibility::Visible;
        visible_in_hierarchy.set_if_neq(VisibleInHierarchy(is_visible));
        // reset "view" visibility here ... if this entity should be drawn a future system should set this to true
        visible_in_view.set_if_neq(VisibleInView(false));
        is_visible
    };

    for child in children_query.get(entity).map_err(drop)?.iter() {
        let _ = propagate_recursive(is_visible, visibility_query, children_query, *child, entity);
    }
    Ok(())
}

/// System updating the visibility of entities each frame.
///
/// The system is part of the [`VisibilitySystems::CheckVisibility`] set. Each frame, it updates the
/// [`ComputedVisibility`] of all entities, and for each view also compute the [`VisibleEntities`]
/// for that view.
pub fn check_visibility(
    mut thread_queues: Local<ThreadLocal<Cell<Vec<Entity>>>>,
    mut view_query: Query<(&mut VisibleEntities, &Frustum, Option<&RenderLayers>), With<Camera>>,
    mut visible_aabb_query: Query<(
        Entity,
        &VisibleInHierarchy,
        &mut VisibleInView,
        Option<&RenderLayers>,
        &Aabb,
        &GlobalTransform,
        Option<&NoFrustumCulling>,
    )>,
    mut visible_no_aabb_query: Query<
        (
            Entity,
            &VisibleInHierarchy,
            &mut VisibleInView,
            Option<&RenderLayers>,
        ),
        Without<Aabb>,
    >,
) {
    for (mut visible_entities, frustum, maybe_view_mask) in &mut view_query {
        let view_mask = maybe_view_mask.copied().unwrap_or_default();

        visible_entities.entities.clear();
        visible_aabb_query.par_iter_mut().for_each(
            |(
                entity,
                visible_in_hierarchy,
                mut visible_in_view,
                maybe_entity_mask,
                model_aabb,
                transform,
                maybe_no_frustum_culling,
            )| {
                // skip computing visibility for entities that are configured to be hidden. is_visible_in_view has already been set to false
                // in visibility_propagate_system
                if !visible_in_hierarchy.get() {
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
                        radius: transform.radius_vec3a(model_aabb.half_extents),
                    };
                    // Do quick sphere-based frustum culling
                    if !frustum.intersects_sphere(&model_sphere, false) {
                        return;
                    }
                    // If we have an aabb, do aabb-based frustum culling
                    if !frustum.intersects_obb(model_aabb, &model, true, false) {
                        return;
                    }
                }

                visible_in_view.set();
                let cell = thread_queues.get_or_default();
                let mut queue = cell.take();
                queue.push(entity);
                cell.set(queue);
            },
        );

        visible_no_aabb_query.par_iter_mut().for_each(
            |(entity, visible_in_hierarchy, mut visible_in_view, maybe_entity_mask)| {
                // skip computing visibility for entities that are configured to be hidden. is_visible_in_view has already been set to false
                // in visibility_propagate_system
                if !visible_in_hierarchy.get() {
                    return;
                }

                let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
                if !view_mask.intersects(&entity_mask) {
                    return;
                }

                visible_in_view.set();
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
        app.add_systems(Update, visibility_propagate_system);

        let root1 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            })
            .id();
        let root1_child1 = app.world.spawn(VisibilityBundle::default()).id();
        let root1_child2 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            })
            .id();
        let root1_child1_grandchild1 = app.world.spawn(VisibilityBundle::default()).id();
        let root1_child2_grandchild1 = app.world.spawn(VisibilityBundle::default()).id();

        app.world
            .entity_mut(root1)
            .push_children(&[root1_child1, root1_child2]);
        app.world
            .entity_mut(root1_child1)
            .push_children(&[root1_child1_grandchild1]);
        app.world
            .entity_mut(root1_child2)
            .push_children(&[root1_child2_grandchild1]);

        let root2 = app.world.spawn(VisibilityBundle::default()).id();
        let root2_child1 = app.world.spawn(VisibilityBundle::default()).id();
        let root2_child2 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            })
            .id();
        let root2_child1_grandchild1 = app.world.spawn(VisibilityBundle::default()).id();
        let root2_child2_grandchild1 = app.world.spawn(VisibilityBundle::default()).id();

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
                .get::<VisibleInHierarchy>()
                .unwrap()
                .get()
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

    #[test]
    fn visibility_propagation_unconditional_visible() {
        let mut app = App::new();
        app.add_systems(Update, visibility_propagate_system);

        let root1 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Visible,
                ..Default::default()
            })
            .id();
        let root1_child1 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Inherited,
                ..Default::default()
            })
            .id();
        let root1_child2 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            })
            .id();
        let root1_child1_grandchild1 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Visible,
                ..Default::default()
            })
            .id();
        let root1_child2_grandchild1 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Visible,
                ..Default::default()
            })
            .id();

        let root2 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Inherited,
                ..Default::default()
            })
            .id();
        let root3 = app
            .world
            .spawn(VisibilityBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            })
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

        app.update();

        let is_visible = |e: Entity| {
            app.world
                .entity(e)
                .get::<VisibleInHierarchy>()
                .unwrap()
                .get()
        };
        assert!(
            is_visible(root1),
            "an unconditionally visible root is visible"
        );
        assert!(
            is_visible(root1_child1),
            "an inheriting child of an unconditionally visible parent is visible"
        );
        assert!(
            !is_visible(root1_child2),
            "a hidden child on an unconditionally visible parent is hidden"
        );
        assert!(
            is_visible(root1_child1_grandchild1),
            "an unconditionally visible child of an inheriting parent is visible"
        );
        assert!(
            is_visible(root1_child2_grandchild1),
            "an unconditionally visible child of a hidden parent is visible"
        );
        assert!(is_visible(root2), "an inheriting root is visible");
        assert!(!is_visible(root3), "a hidden root is hidden");
    }

    #[test]
    fn ensure_visibility_enum_size() {
        use std::mem;
        assert_eq!(1, mem::size_of::<Visibility>());
        assert_eq!(1, mem::size_of::<Option<Visibility>>());
    }
}
