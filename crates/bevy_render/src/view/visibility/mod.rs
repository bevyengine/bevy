#![expect(deprecated)]

mod range;
mod render_layers;

use core::any::TypeId;

use bevy_ecs::entity::EntityHashSet;
pub use range::*;
pub use render_layers::*;

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{change_detection::DetectChangesMut as _, prelude::*, query::QueryFilter};
use bevy_hierarchy::{Children, Parent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystem};
use bevy_utils::{Parallel, TypeIdMap};

use super::NoCpuCulling;
use crate::sync_world::MainEntity;
use crate::{
    camera::{Camera, CameraProjection, Projection},
    mesh::{Mesh, Mesh3d, MeshAabb},
    primitives::{Aabb, Frustum, Sphere},
};

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.
///
/// If an entity is hidden in this way, all [`Children`] (and all of their children and so on) who
/// are set to [`Inherited`](Self::Inherited) will also be hidden.
///
/// This is done by the `visibility_propagate_system` which uses the entity hierarchy and
/// `Visibility` to set the values of each entity's [`InheritedVisibility`] component.
#[derive(Component, Clone, Copy, Reflect, Debug, PartialEq, Eq, Default)]
#[reflect(Component, Default, Debug, PartialEq)]
#[require(InheritedVisibility, ViewVisibility)]
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

impl Visibility {
    /// Toggles between `Visibility::Inherited` and `Visibility::Visible`.
    /// If the value is `Visibility::Hidden`, it remains unaffected.
    #[inline]
    pub fn toggle_inherited_visible(&mut self) {
        *self = match *self {
            Visibility::Inherited => Visibility::Visible,
            Visibility::Visible => Visibility::Inherited,
            _ => *self,
        };
    }
    /// Toggles between `Visibility::Inherited` and `Visibility::Hidden`.
    /// If the value is `Visibility::Visible`, it remains unaffected.
    #[inline]
    pub fn toggle_inherited_hidden(&mut self) {
        *self = match *self {
            Visibility::Inherited => Visibility::Hidden,
            Visibility::Hidden => Visibility::Inherited,
            _ => *self,
        };
    }
    /// Toggles between `Visibility::Visible` and `Visibility::Hidden`.
    /// If the value is `Visibility::Inherited`, it remains unaffected.
    #[inline]
    pub fn toggle_visible_hidden(&mut self) {
        *self = match *self {
            Visibility::Visible => Visibility::Hidden,
            Visibility::Hidden => Visibility::Visible,
            _ => *self,
        };
    }
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
#[reflect(Component, Default, Debug, PartialEq)]
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
#[reflect(Component, Default, Debug, PartialEq)]
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
///   * ~~You may use the [`VisibilityBundle`] to guarantee this.~~ [`VisibilityBundle`] is now deprecated.
///     [`InheritedVisibility`] and [`ViewVisibility`] are automatically inserted whenever [`Visibility`] is inserted.
#[derive(Bundle, Debug, Clone, Default)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `Visibility` component instead. Inserting it will now also insert `InheritedVisibility` and `ViewVisibility` automatically."
)]
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
///     to appear in the reflection of a [`Mesh`] within.
#[derive(Debug, Component, Default, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct NoFrustumCulling;

/// Collection of entities visible from the current view.
///
/// This component contains all entities which are visible from the currently
/// rendered view. The collection is updated automatically by the [`VisibilitySystems::CheckVisibility`]
/// system set. Renderers can use the equivalent [`RenderVisibleEntities`] to optimize rendering of
/// a particular view, to prevent drawing items not visible from that view.
///
/// This component is intended to be attached to the same entity as the [`Camera`] and
/// the [`Frustum`] defining the view.
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug)]
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
}

/// Collection of entities visible from the current view.
///
/// This component is extracted from [`VisibleEntities`].
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct RenderVisibleEntities {
    #[reflect(ignore)]
    pub entities: TypeIdMap<Vec<(Entity, MainEntity)>>,
}

impl RenderVisibleEntities {
    pub fn get<QF>(&self) -> &[(Entity, MainEntity)]
    where
        QF: 'static,
    {
        match self.entities.get(&TypeId::of::<QF>()) {
            Some(entities) => &entities[..],
            None => &[],
        }
    }

    pub fn iter<QF>(&self) -> impl DoubleEndedIterator<Item = &(Entity, MainEntity)>
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
}

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
    /// of each entity and the [`VisibleEntities`] of each view.\
    ///
    /// System order ambiguities between systems in this set are ignored:
    /// the order of systems within this set is irrelevant, as [`check_visibility`]
    /// assumes that its operations are irreversible during the frame.
    CheckVisibility,
    /// Label for the system that runs after visibility checking and marks
    /// entities that have gone from visible to invisible, or vice versa, as
    /// changed.
    VisibilityChangeDetect,
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
        .configure_sets(PostUpdate, CheckVisibility.ambiguous_with(CheckVisibility))
        .configure_sets(PostUpdate, VisibilityChangeDetect.after(CheckVisibility))
        .init_resource::<PreviousVisibleEntities>()
        .add_systems(
            PostUpdate,
            (
                calculate_bounds.in_set(CalculateBounds),
                (visibility_propagate_system, reset_view_visibility).in_set(VisibilityPropagate),
                check_visibility::<With<Mesh3d>>.in_set(CheckVisibility),
                mark_view_visibility_as_changed_if_necessary.in_set(VisibilityChangeDetect),
            ),
        );
    }
}

/// Computes and adds an [`Aabb`] component to entities with a
/// [`Mesh3d`] component and without a [`NoFrustumCulling`] component.
///
/// This system is used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    without_aabb: Query<(Entity, &Mesh3d), (Without<Aabb>, Without<NoFrustumCulling>)>,
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

/// Stores all entities that were visible in the previous frame.
///
/// At the end of visibility checking, we compare the visible entities against
/// these and set the [`ViewVisibility`] change flags accordingly.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct PreviousVisibleEntities(EntityHashSet);

/// Resets the view visibility of every entity.
/// Entities that are visible will be marked as such later this frame
/// by a [`VisibilitySystems::CheckVisibility`] system.
fn reset_view_visibility(
    mut query: Query<(Entity, &mut ViewVisibility)>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
) {
    previous_visible_entities.clear();

    query.iter_mut().for_each(|(entity, mut view_visibility)| {
        // Record the entities that were previously visible.
        if view_visibility.get() {
            previous_visible_entities.insert(entity);
        }

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
        Option<&Projection>,
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

    for (
        view,
        mut visible_entities,
        frustum,
        maybe_view_mask,
        maybe_projection,
        camera,
        no_cpu_culling,
    ) in &mut view_query
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
                        // Only use far culling if we have a projection with some `far` value.
                        let use_far_culling = maybe_projection.is_some_and(|p| p.far().is_some());

                        let world_from_local = transform.affine();
                        let model_sphere = Sphere {
                            center: world_from_local.transform_point3a(model_aabb.center),
                            radius: transform.radius_vec3a(model_aabb.half_extents),
                        };
                        // Do quick sphere-based frustum culling
                        if !frustum.intersects_sphere(&model_sphere, use_far_culling) {
                            return;
                        }
                        // Do aabb-based frustum culling
                        if !frustum.intersects_obb(
                            model_aabb,
                            &world_from_local,
                            true,
                            use_far_culling,
                        ) {
                            return;
                        }
                    }
                }

                // Make sure we don't trigger changed notifications
                // unnecessarily.
                view_visibility.bypass_change_detection().set();

                queue.push(entity);
            },
        );

        visible_entities.clear::<QF>();
        thread_queues.drain_into(visible_entities.get_mut::<QF>());
    }
}

/// A system that marks [`ViewVisibility`] components as changed if their
/// visibility changed this frame.
///
/// Ordinary change detection doesn't work for this use case because we use
/// multiple [`check_visibility`] systems, and visibility is set if *any one* of
/// those systems judges the entity to be visible. Thus, in order to perform
/// change detection, we need this system, which is a follow-up pass that has a
/// global view of whether the entity became visible or not.
fn mark_view_visibility_as_changed_if_necessary(
    mut query: Query<(Entity, &mut ViewVisibility)>,
    previous_visible_entities: Res<PreviousVisibleEntities>,
) {
    for (entity, mut view_visibility) in query.iter_mut() {
        if previous_visible_entities.contains(&entity) != view_visibility.get() {
            view_visibility.set_changed();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_app::prelude::*;
    use bevy_hierarchy::BuildChildren;

    #[test]
    fn visibility_propagation() {
        let mut app = App::new();
        app.add_systems(Update, visibility_propagate_system);

        let root1 = app.world_mut().spawn(Visibility::Hidden).id();
        let root1_child1 = app.world_mut().spawn(Visibility::default()).id();
        let root1_child2 = app.world_mut().spawn(Visibility::Hidden).id();
        let root1_child1_grandchild1 = app.world_mut().spawn(Visibility::default()).id();
        let root1_child2_grandchild1 = app.world_mut().spawn(Visibility::default()).id();

        app.world_mut()
            .entity_mut(root1)
            .add_children(&[root1_child1, root1_child2]);
        app.world_mut()
            .entity_mut(root1_child1)
            .add_children(&[root1_child1_grandchild1]);
        app.world_mut()
            .entity_mut(root1_child2)
            .add_children(&[root1_child2_grandchild1]);

        let root2 = app.world_mut().spawn(Visibility::default()).id();
        let root2_child1 = app.world_mut().spawn(Visibility::default()).id();
        let root2_child2 = app.world_mut().spawn(Visibility::Hidden).id();
        let root2_child1_grandchild1 = app.world_mut().spawn(Visibility::default()).id();
        let root2_child2_grandchild1 = app.world_mut().spawn(Visibility::default()).id();

        app.world_mut()
            .entity_mut(root2)
            .add_children(&[root2_child1, root2_child2]);
        app.world_mut()
            .entity_mut(root2_child1)
            .add_children(&[root2_child1_grandchild1]);
        app.world_mut()
            .entity_mut(root2_child2)
            .add_children(&[root2_child2_grandchild1]);

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

        let root1 = app.world_mut().spawn(Visible).id();
        let root1_child1 = app.world_mut().spawn(Inherited).id();
        let root1_child2 = app.world_mut().spawn(Hidden).id();
        let root1_child1_grandchild1 = app.world_mut().spawn(Visible).id();
        let root1_child2_grandchild1 = app.world_mut().spawn(Visible).id();

        let root2 = app.world_mut().spawn(Inherited).id();
        let root3 = app.world_mut().spawn(Hidden).id();

        app.world_mut()
            .entity_mut(root1)
            .add_children(&[root1_child1, root1_child2]);
        app.world_mut()
            .entity_mut(root1_child1)
            .add_children(&[root1_child1_grandchild1]);
        app.world_mut()
            .entity_mut(root1_child2)
            .add_children(&[root1_child2_grandchild1]);

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

        let id1 = world.spawn(Visibility::default()).id();

        let id2 = world.spawn(Visibility::default()).id();
        world.entity_mut(id1).add_children(&[id2]);

        let id3 = world.spawn(Visibility::Hidden).id();
        world.entity_mut(id2).add_children(&[id3]);

        let id4 = world.spawn(Visibility::default()).id();
        world.entity_mut(id3).add_children(&[id4]);

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
        let child = world.spawn(Visibility::default()).id();
        world.entity_mut(parent).add_children(&[child]);

        schedule.run(&mut world);
        world.clear_trackers();

        let child_visible = world.entity(child).get::<InheritedVisibility>().unwrap().0;
        // defaults to same behavior of parent not found: visible = true
        assert!(child_visible);
    }

    #[test]
    fn ensure_visibility_enum_size() {
        assert_eq!(1, size_of::<Visibility>());
        assert_eq!(1, size_of::<Option<Visibility>>());
    }
}
