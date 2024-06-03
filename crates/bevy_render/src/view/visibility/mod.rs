mod range;
mod render_layers;

use std::any::TypeId;

use bevy_math::Vec3A;
pub use range::*;
pub use render_layers::*;

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_derive::Deref;
use bevy_ecs::{prelude::*, query::QueryFilter};
use bevy_hierarchy::{Children, Parent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{
    components::{GlobalTransform, Transform},
    TransformSystem,
};
use bevy_utils::{Parallel, TypeIdMap};

use crate::{
    camera::{Camera, CameraProjection, CubemapFaceProjections, OmnidirectionalProjection},
    mesh::Mesh,
    primitives::{Aabb, CubemapFrusta, Frustum, Sphere},
};

use super::NoCpuCulling;

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.
///
/// If an entity is hidden in this way, all [`Children`] (and all of their children and so on) who
/// are set to [`Inherited`](Self::Inherited) will also be hidden.
///
/// This is done by the `visibility_propagate_system` which uses the entity hierarchy and
/// `Visibility` to set the values of each entity's [`InheritedVisibility`] component.
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
impl PartialEq<Visibility> for &Visibility {
    #[inline]
    fn eq(&self, other: &Visibility) -> bool {
        // Use the base Visibility == Visibility implementation.
        <Visibility as PartialEq<Visibility>>::eq(*self, other)
    }
}

// Allows `Visibility == &Visibility`
impl PartialEq<&Visibility> for Visibility {
    #[inline]
    fn eq(&self, other: &&Visibility) -> bool {
        // Use the base Visibility == Visibility implementation.
        <Visibility as PartialEq<Visibility>>::eq(self, *other)
    }
}

/// Whether or not an entity is visible in the hierarchy.
/// This will not be accurate until [`VisibilityPropagate`] runs in the [`PostUpdate`] schedule.
///
/// If this is false, then [`ViewVisibility`] should also be false.
///
/// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
#[derive(Component, Deref, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct InheritedVisibility(bool);

impl InheritedVisibility {
    /// An entity that is invisible in the hierarchy.
    pub const HIDDEN: Self = Self(false);
    /// An entity that is visible in the hierarchy.
    pub const VISIBLE: Self = Self(true);

    /// Returns `true` if the entity is visible in the hierarchy.
    /// Otherwise, returns `false`.
    #[inline]
    pub fn get(self) -> bool {
        self.0
    }
}

/// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering.
///
/// Each frame, this will be reset to `false` during [`VisibilityPropagate`] systems in [`PostUpdate`].
/// Later in the frame, systems in [`CheckVisibility`] will mark any visible entities using [`ViewVisibility::set`].
/// Because of this, values of this type will be marked as changed every frame, even when they do not change.
///
/// If you wish to add custom visibility system that sets this value, make sure you add it to the [`CheckVisibility`] set.
///
/// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
/// [`CheckVisibility`]: VisibilitySystems::CheckVisibility
#[derive(Component, Deref, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct ViewVisibility(bool);

impl ViewVisibility {
    /// An entity that cannot be seen from any views.
    pub const HIDDEN: Self = Self(false);

    /// Returns `true` if the entity is visible in any view.
    /// Otherwise, returns `false`.
    #[inline]
    pub fn get(self) -> bool {
        self.0
    }

    /// Sets the visibility to `true`. This should not be considered reversible for a given frame,
    /// as this component tracks whether or not the entity visible in _any_ view.
    ///
    /// This will be automatically reset to `false` every frame in [`VisibilityPropagate`] and then set
    /// to the proper value in [`CheckVisibility`].
    ///
    /// You should only manually set this if you are defining a custom visibility system,
    /// in which case the system should be placed in the [`CheckVisibility`] set.
    /// For normal user-defined entity visibility, see [`Visibility`].
    ///
    /// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
    /// [`CheckVisibility`]: VisibilitySystems::CheckVisibility
    #[inline]
    pub fn set(&mut self) {
        self.0 = true;
    }
}

/// A [`Bundle`] of the [`Visibility`], [`InheritedVisibility`], and [`ViewVisibility`]
/// [`Component`]s, which describe the visibility of an entity.
///
/// * To show or hide an entity, you should set its [`Visibility`].
/// * To get the inherited visibility of an entity, you should get its [`InheritedVisibility`].
/// * For visibility hierarchies to work correctly, you must have both all of [`Visibility`], [`InheritedVisibility`], and [`ViewVisibility`].
///   * You may use the [`VisibilityBundle`] to guarantee this.
#[derive(Bundle, Debug, Clone, Default)]
pub struct VisibilityBundle {
    /// The visibility of the entity.
    pub visibility: Visibility,
    // The inherited visibility of the entity.
    pub inherited_visibility: InheritedVisibility,
    // The computed visibility of the entity.
    pub view_visibility: ViewVisibility,
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
#[reflect(Component, Default)]
pub struct VisibleEntities {
    #[reflect(ignore)]
    pub entities: TypeIdMap<Vec<Entity>>,
}

impl VisibleEntities {
    pub fn get<QF>(&self) -> &[Entity]
    where
        QF: 'static,
    {
        match self.entities.get(&TypeId::of::<QF>()) {
            Some(entities) => &entities[..],
            None => &[],
        }
    }

    pub fn get_mut<QF>(&mut self) -> &mut Vec<Entity>
    where
        QF: 'static,
    {
        self.entities.entry(TypeId::of::<QF>()).or_default()
    }

    pub fn iter<QF>(&self) -> impl DoubleEndedIterator<Item = &Entity>
    where
        QF: 'static,
    {
        self.get::<QF>().iter()
    }

    pub fn len<QF>(&self) -> usize
    where
        QF: 'static,
    {
        self.get::<QF>().len()
    }

    pub fn is_empty<QF>(&self) -> bool
    where
        QF: 'static,
    {
        self.get::<QF>().is_empty()
    }

    pub fn clear<QF>(&mut self)
    where
        QF: 'static,
    {
        self.get_mut::<QF>().clear();
    }

    pub fn push<QF>(&mut self, entity: Entity)
    where
        QF: 'static,
    {
        self.get_mut::<QF>().push(entity);
    }

    pub fn shrink(&mut self) {
        // Check that visible entities capacity() is no more than two times greater than len()
        let capacity = self.entities.capacity();
        let reserved = capacity
            .checked_div(self.entities.len())
            .map_or(0, |reserve| {
                if reserve > 2 {
                    capacity / (reserve / 2)
                } else {
                    capacity
                }
            });

        self.entities.shrink_to(reserved);
    }
}

/// A component that stores which entities are visible from each face of a
/// cubemap.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CubemapVisibleEntities {
    #[reflect(ignore)]
    data: [VisibleEntities; 6],
}

impl CubemapVisibleEntities {
    /// Returns a reference to the list of visible entities from one face of the
    /// cubemap.
    pub fn get(&self, i: usize) -> &VisibleEntities {
        &self.data[i]
    }

    /// Returns a mutable reference to the list of visible entities from one
    /// face of the cubemap.
    pub fn get_mut(&mut self, i: usize) -> &mut VisibleEntities {
        &mut self.data[i]
    }

    /// Iterates over the visible entities of each cubemap face in turn.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleEntities> {
        self.data.iter()
    }

    /// Iterates over the visible entities of each cubemap face in turn,
    /// mutably.
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut VisibleEntities> {
        self.data.iter_mut()
    }

    /// Clears out all entities from all faces of the cubemap in preparation for
    /// a new frame.
    pub fn clear(&mut self) {
        for visible_entities in &mut self.data {
            visible_entities.entities.clear();
        }
    }
}

/// A convenient alias for `With<Handle<Mesh>>`, for use with
/// [`VisibleEntities`].
pub type WithMesh = With<Handle<Mesh>>;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum VisibilitySystems {
    /// Label for the [`calculate_bounds`], `calculate_bounds_2d` and `calculate_bounds_text2d` systems,
    /// calculating and inserting an [`Aabb`] to relevant entities.
    CalculateBounds,
    /// Label for [`update_frusta`] in [`CameraProjectionPlugin`](crate::camera::CameraProjectionPlugin).
    UpdateFrusta,
    /// Label for the system propagating the [`InheritedVisibility`] in a
    /// [`hierarchy`](bevy_hierarchy).
    VisibilityPropagate,
    /// Label for the [`check_visibility`] system updating [`ViewVisibility`]
    /// of each entity and the [`VisibleEntities`] of each view.
    CheckVisibility,
}

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use VisibilitySystems::*;

        app.configure_sets(
            PostUpdate,
            (CalculateBounds, UpdateFrusta, VisibilityPropagate)
                .before(CheckVisibility)
                .after(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            (
                calculate_bounds.in_set(CalculateBounds),
                (visibility_propagate_system, reset_view_visibility).in_set(VisibilityPropagate),
                check_visibility::<WithMesh>.in_set(CheckVisibility),
                update_omnidirectional_camera_frusta.in_set(UpdateFrusta),
                check_omnidirectional_camera_visibility.in_set(CheckVisibility),
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
                commands.entity(entity).try_insert(aabb);
            }
        }
    }
}

/// Updates [`Frustum`].
///
/// This system is used in [`CameraProjectionPlugin`](crate::camera::CameraProjectionPlugin).
pub fn update_frusta<T: Component + CameraProjection + Send + Sync + 'static>(
    mut views: Query<
        (&GlobalTransform, &T, &mut Frustum),
        Or<(Changed<GlobalTransform>, Changed<T>)>,
    >,
) {
    for (transform, projection, mut frustum) in &mut views {
        *frustum = projection.compute_frustum(transform);
    }
}

fn visibility_propagate_system(
    changed: Query<
        (Entity, &Visibility, Option<&Parent>, Option<&Children>),
        (With<InheritedVisibility>, Changed<Visibility>),
    >,
    mut visibility_query: Query<(&Visibility, &mut InheritedVisibility)>,
    children_query: Query<&Children, (With<Visibility>, With<InheritedVisibility>)>,
) {
    for (entity, visibility, parent, children) in &changed {
        let is_visible = match visibility {
            Visibility::Visible => true,
            Visibility::Hidden => false,
            // fall back to true if no parent is found or parent lacks components
            Visibility::Inherited => parent
                .and_then(|p| visibility_query.get(p.get()).ok())
                .map_or(true, |(_, x)| x.get()),
        };
        let (_, mut inherited_visibility) = visibility_query
            .get_mut(entity)
            .expect("With<InheritedVisibility> ensures this query will return a value");

        // Only update the visibility if it has changed.
        // This will also prevent the visibility from propagating multiple times in the same frame
        // if this entity's visibility has been updated recursively by its parent.
        if inherited_visibility.get() != is_visible {
            inherited_visibility.0 = is_visible;

            // Recursively update the visibility of each child.
            for &child in children.into_iter().flatten() {
                let _ =
                    propagate_recursive(is_visible, child, &mut visibility_query, &children_query);
            }
        }
    }
}

fn propagate_recursive(
    parent_is_visible: bool,
    entity: Entity,
    visibility_query: &mut Query<(&Visibility, &mut InheritedVisibility)>,
    children_query: &Query<&Children, (With<Visibility>, With<InheritedVisibility>)>,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    // Get the visibility components for the current entity.
    // If the entity does not have the required components, just return early.
    let (visibility, mut inherited_visibility) = visibility_query.get_mut(entity).map_err(drop)?;

    let is_visible = match visibility {
        Visibility::Visible => true,
        Visibility::Hidden => false,
        Visibility::Inherited => parent_is_visible,
    };

    // Only update the visibility if it has changed.
    if inherited_visibility.get() != is_visible {
        inherited_visibility.0 = is_visible;

        // Recursively update the visibility of each child.
        for &child in children_query.get(entity).ok().into_iter().flatten() {
            let _ = propagate_recursive(is_visible, child, visibility_query, children_query);
        }
    }

    Ok(())
}

/// Resets the view visibility of every entity.
/// Entities that are visible will be marked as such later this frame
/// by a [`VisibilitySystems::CheckVisibility`] system.
fn reset_view_visibility(mut query: Query<&mut ViewVisibility>) {
    query.iter_mut().for_each(|mut view_visibility| {
        // NOTE: We do not use `set_if_neq` here, as we don't care about
        // change detection for view visibility, and adding a branch to every
        // loop iteration would pessimize performance.
        *view_visibility.bypass_change_detection() = ViewVisibility::HIDDEN;
    });
}

/// System updating the visibility of entities each frame.
///
/// The system is part of the [`VisibilitySystems::CheckVisibility`] set. Each
/// frame, it updates the [`ViewVisibility`] of all entities, and for each view
/// also compute the [`VisibleEntities`] for that view.
///
/// This system needs to be run for each type of renderable entity. If you add a
/// new type of renderable entity, you'll need to add an instantiation of this
/// system to the [`VisibilitySystems::CheckVisibility`] set so that Bevy will
/// detect visibility properly for those entities.
pub fn check_visibility<QF>(
    mut thread_queues: Local<Parallel<Vec<Entity>>>,
    mut view_query: Query<(
        Entity,
        &mut VisibleEntities,
        &Frustum,
        Option<&RenderLayers>,
        &Camera,
        Has<NoCpuCulling>,
    )>,
    mut visible_aabb_query: Query<
        (
            Entity,
            &InheritedVisibility,
            &mut ViewVisibility,
            Option<&RenderLayers>,
            Option<&Aabb>,
            &GlobalTransform,
            Has<NoFrustumCulling>,
            Has<VisibilityRange>,
        ),
        QF,
    >,
    visible_entity_ranges: Option<Res<VisibleEntityRanges>>,
) where
    QF: QueryFilter + 'static,
{
    let visible_entity_ranges = visible_entity_ranges.as_deref();

    for (view, mut visible_entities, frustum, maybe_view_mask, camera, no_cpu_culling) in
        &mut view_query
    {
        if !camera.is_active {
            continue;
        }

        let view_mask = maybe_view_mask.unwrap_or_default();

        visible_aabb_query.par_iter_mut().for_each_init(
            || thread_queues.borrow_local_mut(),
            |queue, query_item| {
                let (
                    entity,
                    inherited_visibility,
                    mut view_visibility,
                    maybe_entity_mask,
                    maybe_model_aabb,
                    transform,
                    no_frustum_culling,
                    has_visibility_range,
                ) = query_item;

                // Skip computing visibility for entities that are configured to be hidden.
                // ViewVisibility has already been reset in `reset_view_visibility`.
                if !inherited_visibility.get() {
                    return;
                }

                let entity_mask = maybe_entity_mask.unwrap_or_default();
                if !view_mask.intersects(entity_mask) {
                    return;
                }

                // If outside of the visibility range, cull.
                if has_visibility_range
                    && visible_entity_ranges.is_some_and(|visible_entity_ranges| {
                        !visible_entity_ranges.entity_is_in_range_of_view(entity, view)
                    })
                {
                    return;
                }

                // If we have an aabb, do frustum culling
                if !no_frustum_culling && !no_cpu_culling {
                    if let Some(model_aabb) = maybe_model_aabb {
                        let world_from_local = transform.affine();
                        let model_sphere = Sphere {
                            center: world_from_local.transform_point3a(model_aabb.center),
                            radius: transform.radius_vec3a(model_aabb.half_extents),
                        };
                        // Do quick sphere-based frustum culling
                        if !frustum.intersects_sphere(&model_sphere, false) {
                            return;
                        }
                        // Do aabb-based frustum culling
                        if !frustum.intersects_obb(model_aabb, &world_from_local, true, false) {
                            return;
                        }
                    }
                }

                view_visibility.set();
                queue.push(entity);
            },
        );

        visible_entities.clear::<QF>();
        thread_queues.drain_into(visible_entities.get_mut::<QF>());
    }
}

/// A system that checks visibility for every face of an omnidirectional camera.
///
/// This has to be done separately from the normal [`check_visibility`] because
/// omnidirectional cameras have six individual sub-cameras, one for each face,
/// and each face needs to know the entities visible from that face.
pub fn check_omnidirectional_camera_visibility(
    mut visible_entity_query: Query<(
        Entity,
        &InheritedVisibility,
        &mut ViewVisibility,
        Option<&RenderLayers>,
        Option<&Aabb>,
        Option<&GlobalTransform>,
        Has<VisibilityRange>,
    )>,
    mut omnidirectional_camera_query: Query<(
        &mut CubemapVisibleEntities,
        &CubemapFrusta,
        &GlobalTransform,
        &OmnidirectionalProjection,
        Option<&RenderLayers>,
    )>,
    visible_entity_ranges: Option<Res<VisibleEntityRanges>>,
) {
    for (
        mut cubemap_visible_entities,
        cubemap_frusta,
        cubemap_transform,
        cubemap_projection,
        maybe_render_layers,
    ) in omnidirectional_camera_query.iter_mut()
    {
        cubemap_visible_entities.clear();

        check_cubemap_mesh_visibility(
            &mut visible_entity_query,
            &mut cubemap_visible_entities,
            cubemap_frusta,
            cubemap_transform,
            maybe_render_layers.unwrap_or_default(),
            cubemap_projection.far,
            visible_entity_ranges.as_deref(),
        );
    }
}

/// A helper function that determines which entities are visible from one face
/// of a cubemap.
///
/// This is used by [`check_omnidirectional_camera_visibility`] as well as the
/// point light shadow map logic.
pub fn check_cubemap_mesh_visibility<QF>(
    visible_entity_query: &mut Query<
        (
            Entity,
            &InheritedVisibility,
            &mut ViewVisibility,
            Option<&RenderLayers>,
            Option<&Aabb>,
            Option<&GlobalTransform>,
            Has<VisibilityRange>,
        ),
        QF,
    >,
    cubemap_visible_entities: &mut CubemapVisibleEntities,
    cubemap_frusta: &CubemapFrusta,
    transform: &GlobalTransform,
    view_mask: &RenderLayers,
    range: f32,
    visible_entity_ranges: Option<&VisibleEntityRanges>,
) where
    QF: QueryFilter,
{
    let light_sphere = Sphere {
        center: Vec3A::from(transform.translation()),
        radius: range,
    };

    for (
        entity,
        inherited_visibility,
        mut view_visibility,
        maybe_entity_mask,
        maybe_aabb,
        maybe_transform,
        has_visibility_range,
    ) in visible_entity_query
    {
        if !inherited_visibility.get() {
            continue;
        }

        let entity_mask = maybe_entity_mask.unwrap_or_default();
        if !view_mask.intersects(entity_mask) {
            continue;
        }

        // Check visibility ranges.
        if has_visibility_range
            && visible_entity_ranges.is_some_and(|visible_entity_ranges| {
                !visible_entity_ranges.entity_is_in_range_of_any_view(entity)
            })
        {
            continue;
        }

        // If we have an aabb and transform, do frustum culling
        if let (Some(aabb), Some(transform)) = (maybe_aabb, maybe_transform) {
            let model_to_world = transform.affine();
            // Do a cheap sphere vs obb test to prune out most meshes outside the sphere of the light
            if !light_sphere.intersects_obb(aabb, &model_to_world) {
                continue;
            }

            for (frustum, visible_entities) in cubemap_frusta
                .iter()
                .zip(cubemap_visible_entities.iter_mut())
            {
                if frustum.intersects_obb(aabb, &model_to_world, true, true) {
                    view_visibility.set();
                    visible_entities.push::<WithMesh>(entity);
                }
            }
        } else {
            view_visibility.set();
            for visible_entities in cubemap_visible_entities.iter_mut() {
                visible_entities.push::<WithMesh>(entity);
            }
        }
    }

    for visible_entities in cubemap_visible_entities.iter_mut() {
        visible_entities.shrink();
    }
}

pub fn update_omnidirectional_camera_frusta(
    mut omnidirectional_camera_query: Query<(
        &mut CubemapFrusta,
        &GlobalTransform,
        &OmnidirectionalProjection,
    )>,
) {
    for (mut cubemap_frusta, transform, projection) in omnidirectional_camera_query.iter_mut() {
        let face_projections = CubemapFaceProjections::new(projection.near);
        update_cubemap_frusta(
            transform,
            &mut cubemap_frusta,
            &face_projections,
            projection.far,
        );
    }
}

pub fn update_cubemap_frusta(
    transform: &GlobalTransform,
    cubemap_frusta: &mut CubemapFrusta,
    cubemap_face_projections: &CubemapFaceProjections,
    range: f32,
) {
    let CubemapFaceProjections {
        rotations: ref view_rotations,
        projection: ref clip_from_view,
    } = *cubemap_face_projections;

    // ignore scale because we don't want to effectively scale light radius and range
    // by applying those as a view transform to shadow map rendering of objects
    // and ignore rotation because we want the shadow map projections to align with the axes
    let view_translation = Transform::from_translation(transform.translation());
    let view_backward = transform.back();

    for (view_rotation, frustum) in view_rotations.iter().zip(cubemap_frusta.iter_mut()) {
        let world_from_view = view_translation * *view_rotation;
        let clip_from_world = *clip_from_view * world_from_view.compute_matrix().inverse();

        *frustum = Frustum::from_clip_from_world_custom_far(
            &clip_from_world,
            &transform.translation(),
            &view_backward,
            range,
        );
    }
}

#[cfg(test)]
mod test {
    use bevy_app::prelude::*;
    use bevy_ecs::prelude::*;

    use super::*;

    use bevy_hierarchy::BuildWorldChildren;

    fn visibility_bundle(visibility: Visibility) -> VisibilityBundle {
        VisibilityBundle {
            visibility,
            ..Default::default()
        }
    }

    #[test]
    fn visibility_propagation() {
        let mut app = App::new();
        app.add_systems(Update, visibility_propagate_system);

        let root1 = app
            .world_mut()
            .spawn(visibility_bundle(Visibility::Hidden))
            .id();
        let root1_child1 = app.world_mut().spawn(VisibilityBundle::default()).id();
        let root1_child2 = app
            .world_mut()
            .spawn(visibility_bundle(Visibility::Hidden))
            .id();
        let root1_child1_grandchild1 = app.world_mut().spawn(VisibilityBundle::default()).id();
        let root1_child2_grandchild1 = app.world_mut().spawn(VisibilityBundle::default()).id();

        app.world_mut()
            .entity_mut(root1)
            .push_children(&[root1_child1, root1_child2]);
        app.world_mut()
            .entity_mut(root1_child1)
            .push_children(&[root1_child1_grandchild1]);
        app.world_mut()
            .entity_mut(root1_child2)
            .push_children(&[root1_child2_grandchild1]);

        let root2 = app.world_mut().spawn(VisibilityBundle::default()).id();
        let root2_child1 = app.world_mut().spawn(VisibilityBundle::default()).id();
        let root2_child2 = app
            .world_mut()
            .spawn(visibility_bundle(Visibility::Hidden))
            .id();
        let root2_child1_grandchild1 = app.world_mut().spawn(VisibilityBundle::default()).id();
        let root2_child2_grandchild1 = app.world_mut().spawn(VisibilityBundle::default()).id();

        app.world_mut()
            .entity_mut(root2)
            .push_children(&[root2_child1, root2_child2]);
        app.world_mut()
            .entity_mut(root2_child1)
            .push_children(&[root2_child1_grandchild1]);
        app.world_mut()
            .entity_mut(root2_child2)
            .push_children(&[root2_child2_grandchild1]);

        app.update();

        let is_visible = |e: Entity| {
            app.world()
                .entity(e)
                .get::<InheritedVisibility>()
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
        use Visibility::{Hidden, Inherited, Visible};

        let mut app = App::new();
        app.add_systems(Update, visibility_propagate_system);

        let root1 = app.world_mut().spawn(visibility_bundle(Visible)).id();
        let root1_child1 = app.world_mut().spawn(visibility_bundle(Inherited)).id();
        let root1_child2 = app.world_mut().spawn(visibility_bundle(Hidden)).id();
        let root1_child1_grandchild1 = app.world_mut().spawn(visibility_bundle(Visible)).id();
        let root1_child2_grandchild1 = app.world_mut().spawn(visibility_bundle(Visible)).id();

        let root2 = app.world_mut().spawn(visibility_bundle(Inherited)).id();
        let root3 = app.world_mut().spawn(visibility_bundle(Hidden)).id();

        app.world_mut()
            .entity_mut(root1)
            .push_children(&[root1_child1, root1_child2]);
        app.world_mut()
            .entity_mut(root1_child1)
            .push_children(&[root1_child1_grandchild1]);
        app.world_mut()
            .entity_mut(root1_child2)
            .push_children(&[root1_child2_grandchild1]);

        app.update();

        let is_visible = |e: Entity| {
            app.world()
                .entity(e)
                .get::<InheritedVisibility>()
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
    fn visibility_propagation_change_detection() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(visibility_propagate_system);

        // Set up an entity hierarchy.

        let id1 = world.spawn(VisibilityBundle::default()).id();

        let id2 = world.spawn(VisibilityBundle::default()).id();
        world.entity_mut(id1).push_children(&[id2]);

        let id3 = world.spawn(visibility_bundle(Visibility::Hidden)).id();
        world.entity_mut(id2).push_children(&[id3]);

        let id4 = world.spawn(VisibilityBundle::default()).id();
        world.entity_mut(id3).push_children(&[id4]);

        // Test the hierarchy.

        // Make sure the hierarchy is up-to-date.
        schedule.run(&mut world);
        world.clear_trackers();

        let mut q = world.query::<Ref<InheritedVisibility>>();

        assert!(!q.get(&world, id1).unwrap().is_changed());
        assert!(!q.get(&world, id2).unwrap().is_changed());
        assert!(!q.get(&world, id3).unwrap().is_changed());
        assert!(!q.get(&world, id4).unwrap().is_changed());

        world.clear_trackers();
        world.entity_mut(id1).insert(Visibility::Hidden);
        schedule.run(&mut world);

        assert!(q.get(&world, id1).unwrap().is_changed());
        assert!(q.get(&world, id2).unwrap().is_changed());
        assert!(!q.get(&world, id3).unwrap().is_changed());
        assert!(!q.get(&world, id4).unwrap().is_changed());

        world.clear_trackers();
        schedule.run(&mut world);

        assert!(!q.get(&world, id1).unwrap().is_changed());
        assert!(!q.get(&world, id2).unwrap().is_changed());
        assert!(!q.get(&world, id3).unwrap().is_changed());
        assert!(!q.get(&world, id4).unwrap().is_changed());

        world.clear_trackers();
        world.entity_mut(id3).insert(Visibility::Inherited);
        schedule.run(&mut world);

        assert!(!q.get(&world, id1).unwrap().is_changed());
        assert!(!q.get(&world, id2).unwrap().is_changed());
        assert!(!q.get(&world, id3).unwrap().is_changed());
        assert!(!q.get(&world, id4).unwrap().is_changed());

        world.clear_trackers();
        world.entity_mut(id2).insert(Visibility::Visible);
        schedule.run(&mut world);

        assert!(!q.get(&world, id1).unwrap().is_changed());
        assert!(q.get(&world, id2).unwrap().is_changed());
        assert!(q.get(&world, id3).unwrap().is_changed());
        assert!(q.get(&world, id4).unwrap().is_changed());

        world.clear_trackers();
        schedule.run(&mut world);

        assert!(!q.get(&world, id1).unwrap().is_changed());
        assert!(!q.get(&world, id2).unwrap().is_changed());
        assert!(!q.get(&world, id3).unwrap().is_changed());
        assert!(!q.get(&world, id4).unwrap().is_changed());
    }

    #[test]
    fn visibility_propagation_with_invalid_parent() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(visibility_propagate_system);

        let parent = world.spawn(()).id();
        let child = world.spawn(VisibilityBundle::default()).id();
        world.entity_mut(parent).push_children(&[child]);

        schedule.run(&mut world);
        world.clear_trackers();

        let child_visible = world.entity(child).get::<InheritedVisibility>().unwrap().0;
        // defaults to same behavior of parent not found: visible = true
        assert!(child_visible);
    }

    #[test]
    fn ensure_visibility_enum_size() {
        use std::mem;
        assert_eq!(1, mem::size_of::<Visibility>());
        assert_eq!(1, mem::size_of::<Option<Visibility>>());
    }
}
