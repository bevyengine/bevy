mod render_layers;

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

bitflags::bitflags! {
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(super) struct ComputedVisibilityFlags: u8 {
        const VISIBLE_IN_VIEW = 1 << 0;
        const VISIBLE_IN_HIERARCHY = 1 << 1;
    }
}
bevy_reflect::impl_reflect_value!((in bevy_render::view) ComputedVisibilityFlags);

/// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
#[derive(Component, Clone, Reflect, Debug, Eq, PartialEq)]
#[reflect(Component, Default)]
pub struct ComputedVisibility {
    flags: ComputedVisibilityFlags,
}

impl Default for ComputedVisibility {
    fn default() -> Self {
        Self::HIDDEN
    }
}

impl ComputedVisibility {
    /// A [`ComputedVisibility`], set as invisible.
    pub const HIDDEN: Self = ComputedVisibility {
        flags: ComputedVisibilityFlags::empty(),
    };

    /// Whether this entity is visible to something this frame. This is true if and only if [`Self::is_visible_in_hierarchy`] and [`Self::is_visible_in_view`]
    /// are true. This is the canonical method to call to determine if an entity should be drawn.
    /// This value is updated in [`PostUpdate`] by the [`VisibilitySystems::CheckVisibility`] system set.
    /// Reading it during [`Update`](bevy_app::Update) will yield the value from the previous frame.
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.flags.bits() == ComputedVisibilityFlags::all().bits()
    }

    /// Whether this entity is visible in the entity hierarchy, which is determined by the [`Visibility`] component.
    /// This takes into account "visibility inheritance". If any of this entity's ancestors (see [`Parent`]) are hidden, this entity
    /// will be hidden as well. This value is updated in the [`VisibilitySystems::VisibilityPropagate`], which lives in the [`PostUpdate`] schedule.
    #[inline]
    pub fn is_visible_in_hierarchy(&self) -> bool {
        self.flags
            .contains(ComputedVisibilityFlags::VISIBLE_IN_HIERARCHY)
    }

    /// Whether this entity is visible in _any_ view (Cameras, Lights, etc). Each entity type (and view type) should choose how to set this
    /// value. For cameras and drawn entities, this will take into account [`RenderLayers`].
    ///
    /// This value is reset to `false` every frame in [`VisibilitySystems::VisibilityPropagate`] during [`PostUpdate`].
    /// Each entity type then chooses how to set this field in the [`VisibilitySystems::CheckVisibility`] system set, in [`PostUpdate`].
    /// Meshes might use frustum culling to decide if they are visible in a view.
    /// Other entities might just set this to `true` every frame.
    #[inline]
    pub fn is_visible_in_view(&self) -> bool {
        self.flags
            .contains(ComputedVisibilityFlags::VISIBLE_IN_VIEW)
    }

    /// Sets `is_visible_in_view` to `true`. This is not reversible for a given frame, as it encodes whether or not this is visible in
    /// _any_ view. This will be automatically reset to `false` every frame in [`VisibilitySystems::VisibilityPropagate`] and then set
    /// to the proper value in [`VisibilitySystems::CheckVisibility`]. This should _only_ be set in systems with the [`VisibilitySystems::CheckVisibility`]
    /// label. Don't call this unless you are defining a custom visibility system. For normal user-defined entity visibility, see [`Visibility`].
    #[inline]
    pub fn set_visible_in_view(&mut self) {
        self.flags.insert(ComputedVisibilityFlags::VISIBLE_IN_VIEW);
    }

    #[inline]
    fn reset(&mut self, visible_in_hierarchy: bool) {
        self.flags = if visible_in_hierarchy {
            ComputedVisibilityFlags::VISIBLE_IN_HIERARCHY
        } else {
            ComputedVisibilityFlags::empty()
        };
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
    /// The computed visibility of the entity.
    pub computed: ComputedVisibility,
}

/// Use this component to opt-out of built-in frustum culling for entities, see
/// [`Frustum`].
///
/// It can be used for example:
/// - when a [`Mesh`] is updated but its [`Aabb`] is not, which might happen with animations,
/// - when using some light effects, like wanting a [`Mesh`] out of the [`Frustum`]
/// to appear in the reflection of a [`Mesh`] within.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct NoFrustumCulling;

/// Collection of entities visible from the current view.
///
/// This component contains all entities which are visible from the currently
/// rendered view. The collection is updated automatically by the [`VisibilitySystems::CheckVisibility`]
/// system set, and renderers can use it to optimize rendering of a particular view, to
/// prevent drawing items not visible from that view.
///
/// This component is intended to be attached to the same entity as the [`Camera`] and
/// the [`Frustum`] defining the view.
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
    /// Label for the [`calculate_bounds`] and `calculate_bounds_2d` systems,
    /// calculating and inserting an [`Aabb`] to relevant entities.
    CalculateBounds,
    /// Label for the [`apply_deferred`] call after [`VisibilitySystems::CalculateBounds`]
    CalculateBoundsFlush,
    /// Label for the [`update_frusta<OrthographicProjection>`] system.
    UpdateOrthographicFrusta,
    /// Label for the [`update_frusta<PerspectiveProjection>`] system.
    UpdatePerspectiveFrusta,
    /// Label for the [`update_frusta<Projection>`] system.
    UpdateProjectionFrusta,
    /// Label for the system propagating the [`ComputedVisibility`] in a
    /// [`hierarchy`](bevy_hierarchy).
    VisibilityPropagate,
    /// Label for the [`check_visibility`] system updating [`ComputedVisibility`]
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

/// Computes and adds an [`Aabb`] component to entities with a
/// [`Handle<Mesh>`](Mesh) component and without a [`NoFrustumCulling`] component.
///
/// This system is used in system set [`VisibilitySystems::CalculateBounds`].
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

/// Updates [`Frustum`].
///
/// This system is used in system sets [`VisibilitySystems::UpdateProjectionFrusta`],
/// [`VisibilitySystems::UpdatePerspectiveFrusta`], and
/// [`VisibilitySystems::UpdateOrthographicFrusta`].
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
            &mut ComputedVisibility,
            Entity,
        ),
        Without<Parent>,
    >,
    mut visibility_query: Query<(&Visibility, &mut ComputedVisibility, &Parent)>,
    children_query: Query<&Children, (With<Parent>, With<Visibility>, With<ComputedVisibility>)>,
) {
    for (children, visibility, mut computed_visibility, entity) in root_query.iter_mut() {
        // reset "view" visibility here ... if this entity should be drawn a future system should set this to true
        computed_visibility
            .reset(visibility == Visibility::Inherited || visibility == Visibility::Visible);
        if let Some(children) = children {
            for child in children.iter() {
                let _ = propagate_recursive(
                    computed_visibility.is_visible_in_hierarchy(),
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
        let visible_in_hierarchy = (parent_visible && visibility == Visibility::Inherited)
            || visibility == Visibility::Visible;
        // reset "view" visibility here ... if this entity should be drawn a future system should set this to true
        computed_visibility.reset(visible_in_hierarchy);
        visible_in_hierarchy
    };

    for child in children_query.get(entity).map_err(drop)?.iter() {
        let _ = propagate_recursive(is_visible, visibility_query, children_query, *child, entity);
    }
    Ok(())
}

/// Updates the visibility of entities each frame.
///
/// This system is part of the [`VisibilitySystems::CheckVisibility`] set. Each frame, it updates the
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
        visible_aabb_query.par_iter_mut().for_each(
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
                    let model = transform.affine();
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

                computed_visibility.set_visible_in_view();
                let cell = thread_queues.get_or_default();
                let mut queue = cell.take();
                queue.push(entity);
                cell.set(queue);
            },
        );

        visible_no_aabb_query.par_iter_mut().for_each(
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

                computed_visibility.set_visible_in_view();
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
            .spawn((Visibility::Hidden, ComputedVisibility::default()))
            .id();
        let root1_child1 = app
            .world
            .spawn((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root1_child2 = app
            .world
            .spawn((Visibility::Hidden, ComputedVisibility::default()))
            .id();
        let root1_child1_grandchild1 = app
            .world
            .spawn((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root1_child2_grandchild1 = app
            .world
            .spawn((Visibility::default(), ComputedVisibility::default()))
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
            .spawn((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root2_child1 = app
            .world
            .spawn((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root2_child2 = app
            .world
            .spawn((Visibility::Hidden, ComputedVisibility::default()))
            .id();
        let root2_child1_grandchild1 = app
            .world
            .spawn((Visibility::default(), ComputedVisibility::default()))
            .id();
        let root2_child2_grandchild1 = app
            .world
            .spawn((Visibility::default(), ComputedVisibility::default()))
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
                .is_visible_in_hierarchy()
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
            .spawn((Visibility::Visible, ComputedVisibility::default()))
            .id();
        let root1_child1 = app
            .world
            .spawn((Visibility::Inherited, ComputedVisibility::default()))
            .id();
        let root1_child2 = app
            .world
            .spawn((Visibility::Hidden, ComputedVisibility::default()))
            .id();
        let root1_child1_grandchild1 = app
            .world
            .spawn((Visibility::Visible, ComputedVisibility::default()))
            .id();
        let root1_child2_grandchild1 = app
            .world
            .spawn((Visibility::Visible, ComputedVisibility::default()))
            .id();

        let root2 = app
            .world
            .spawn((Visibility::Inherited, ComputedVisibility::default()))
            .id();
        let root3 = app
            .world
            .spawn((Visibility::Hidden, ComputedVisibility::default()))
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
                .get::<ComputedVisibility>()
                .unwrap()
                .is_visible_in_hierarchy()
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
