mod range;
mod render_layers;

use core::any::TypeId;

use bevy_ecs::entity::{EntityHashMap, EntityHashSet};
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::DeferredWorld;
use derive_more::derive::{Deref, DerefMut};
pub use range::*;
pub use render_layers::*;

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_ecs::{hierarchy::validate_parent_has_component, prelude::*};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystems};
use bevy_utils::{Parallel, TypeIdMap};
use smallvec::SmallVec;

use crate::{
    camera::Camera,
    primitives::{Aabb, Frustum, MeshAabb, Sphere},
    Projection,
};
use bevy_mesh::{mark_3d_meshes_as_changed_if_their_assets_changed, Mesh, Mesh2d, Mesh3d};

#[derive(Component, Default)]
pub struct NoCpuCulling;

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.
///
/// If an entity is hidden in this way, all [`Children`] (and all of their children and so on) who
/// are set to [`Inherited`](Self::Inherited) will also be hidden.
///
/// This is done by the `visibility_propagate_system` which uses the entity hierarchy and
/// `Visibility` to set the values of each entity's [`InheritedVisibility`] component.
#[derive(Component, Clone, Copy, Reflect, Debug, PartialEq, Eq, Default)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(InheritedVisibility, ViewVisibility)]
pub enum Visibility {
    /// An entity with `Visibility::Inherited` will inherit the Visibility of its [`ChildOf`] target.
    ///
    /// A root-level entity that is set to `Inherited` will be visible.
    #[default]
    Inherited,
    /// An entity with `Visibility::Hidden` will be unconditionally hidden.
    Hidden,
    /// An entity with `Visibility::Visible` will be unconditionally visible.
    ///
    /// Note that an entity with `Visibility::Visible` will be visible regardless of whether the
    /// [`ChildOf`] target entity is hidden.
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
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[component(on_insert = validate_parent_has_component::<Self>)]
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

/// A bucket into which we group entities for the purposes of visibility.
///
/// Bevy's various rendering subsystems (3D, 2D, etc.) want to be able to
/// quickly winnow the set of entities to only those that the subsystem is
/// tasked with rendering, to avoid spending time examining irrelevant entities.
/// At the same time, Bevy wants the [`check_visibility`] system to determine
/// all entities' visibilities at the same time, regardless of what rendering
/// subsystem is responsible for drawing them. Additionally, your application
/// may want to add more types of renderable objects that Bevy determines
/// visibility for just as it does for Bevy's built-in objects.
///
/// The solution to this problem is *visibility classes*. A visibility class is
/// a type, typically the type of a component, that represents the subsystem
/// that renders it: for example, `Mesh3d`, `Mesh2d`, and `Sprite`. The
/// [`VisibilityClass`] component stores the visibility class or classes that
/// the entity belongs to. (Generally, an object will belong to only one
/// visibility class, but in rare cases it may belong to multiple.)
///
/// When adding a new renderable component, you'll typically want to write an
/// add-component hook that adds the type ID of that component to the
/// [`VisibilityClass`] array. See `custom_phase_item` for an example.
//
// Note: This can't be a `ComponentId` because the visibility classes are copied
// into the render world, and component IDs are per-world.
#[derive(Clone, Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Default, Clone)]
pub struct VisibilityClass(pub SmallVec<[TypeId; 1]>);

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
#[reflect(Component, Default, Debug, PartialEq, Clone)]
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

/// Use this component to opt-out of built-in frustum culling for entities, see
/// [`Frustum`].
///
/// It can be used for example:
/// - when a [`Mesh`] is updated but its [`Aabb`] is not, which might happen with animations,
/// - when using some light effects, like wanting a [`Mesh`] out of the [`Frustum`]
///   to appear in the reflection of a [`Mesh`] within.
#[derive(Debug, Component, Default, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct NoFrustumCulling;

/// Collection of entities visible from the current view.
///
/// This component contains all entities which are visible from the currently
/// rendered view. The collection is updated automatically by the [`VisibilitySystems::CheckVisibility`]
/// system set. Renderers can use the equivalent `RenderVisibleEntities` to optimize rendering of
/// a particular view, to prevent drawing items not visible from that view.
///
/// This component is intended to be attached to the same entity as the [`Camera`] and
/// the [`Frustum`] defining the view.
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VisibleEntities {
    #[reflect(ignore, clone)]
    pub entities: TypeIdMap<Vec<Entity>>,
}

impl VisibleEntities {
    pub fn get(&self, type_id: TypeId) -> &[Entity] {
        match self.entities.get(&type_id) {
            Some(entities) => &entities[..],
            None => &[],
        }
    }

    pub fn get_mut(&mut self, type_id: TypeId) -> &mut Vec<Entity> {
        self.entities.entry(type_id).or_default()
    }

    pub fn iter(&self, type_id: TypeId) -> impl DoubleEndedIterator<Item = &Entity> {
        self.get(type_id).iter()
    }

    pub fn len(&self, type_id: TypeId) -> usize {
        self.get(type_id).len()
    }

    pub fn is_empty(&self, type_id: TypeId) -> bool {
        self.get(type_id).is_empty()
    }

    pub fn clear(&mut self, type_id: TypeId) {
        self.get_mut(type_id).clear();
    }

    pub fn clear_all(&mut self) {
        // Don't just nuke the hash table; we want to reuse allocations.
        for entities in self.entities.values_mut() {
            entities.clear();
        }
    }

    pub fn push(&mut self, entity: Entity, type_id: TypeId) {
        self.get_mut(type_id).push(entity);
    }
}

/// Collection of mesh entities visible for 3D lighting.
///
/// This component contains all mesh entities visible from the current light view.
/// The collection is updated automatically by `bevy_pbr::SimulationLightSystems`.
#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Default, Clone)]
pub struct VisibleMeshEntities {
    #[reflect(ignore, clone)]
    pub entities: Vec<Entity>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct CubemapVisibleEntities {
    #[reflect(ignore, clone)]
    data: [VisibleMeshEntities; 6],
}

impl CubemapVisibleEntities {
    pub fn get(&self, i: usize) -> &VisibleMeshEntities {
        &self.data[i]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut VisibleMeshEntities {
        &mut self.data[i]
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleMeshEntities> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut VisibleMeshEntities> {
        self.data.iter_mut()
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct CascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore, clone)]
    pub entities: EntityHashMap<Vec<VisibleMeshEntities>>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum VisibilitySystems {
    /// Label for the [`calculate_bounds`], `calculate_bounds_2d` and `calculate_bounds_text2d` systems,
    /// calculating and inserting an [`Aabb`] to relevant entities.
    CalculateBounds,
    /// Label for [`update_frusta`] in [`CameraProjectionPlugin`](crate::CameraProjectionPlugin).
    UpdateFrusta,
    /// Label for the system propagating the [`InheritedVisibility`] in a
    /// [`ChildOf`] / [`Children`] hierarchy.
    VisibilityPropagate,
    /// Label for the [`check_visibility`] system updating [`ViewVisibility`]
    /// of each entity and the [`VisibleEntities`] of each view.\
    ///
    /// System order ambiguities between systems in this set are ignored:
    /// the order of systems within this set is irrelevant, as [`check_visibility`]
    /// assumes that its operations are irreversible during the frame.
    CheckVisibility,
    /// Label for the `mark_newly_hidden_entities_invisible` system, which sets
    /// [`ViewVisibility`] to [`ViewVisibility::HIDDEN`] for entities that no
    /// view has marked as visible.
    MarkNewlyHiddenEntitiesInvisible,
}

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use VisibilitySystems::*;

        app.register_required_components::<Mesh3d, Visibility>()
            .register_required_components::<Mesh3d, VisibilityClass>()
            .register_required_components::<Mesh2d, Visibility>()
            .register_required_components::<Mesh2d, VisibilityClass>()
            .configure_sets(
                PostUpdate,
                (
                    CalculateBounds
                        .ambiguous_with(mark_3d_meshes_as_changed_if_their_assets_changed),
                    UpdateFrusta,
                    VisibilityPropagate,
                )
                    .before(CheckVisibility)
                    .after(TransformSystems::Propagate),
            )
            .configure_sets(
                PostUpdate,
                MarkNewlyHiddenEntitiesInvisible.after(CheckVisibility),
            )
            .init_resource::<PreviousVisibleEntities>()
            .add_systems(
                PostUpdate,
                (
                    calculate_bounds.in_set(CalculateBounds),
                    (visibility_propagate_system, reset_view_visibility)
                        .in_set(VisibilityPropagate),
                    check_visibility.in_set(CheckVisibility),
                    mark_newly_hidden_entities_invisible.in_set(MarkNewlyHiddenEntitiesInvisible),
                ),
            );
        app.world_mut()
            .register_component_hooks::<Mesh3d>()
            .on_add(add_visibility_class::<Mesh3d>);
        app.world_mut()
            .register_component_hooks::<Mesh2d>()
            .on_add(add_visibility_class::<Mesh2d>);
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
        if let Some(mesh) = meshes.get(mesh_handle)
            && let Some(aabb) = mesh.compute_aabb()
        {
            commands.entity(entity).try_insert(aabb);
        }
    }
}

/// Updates [`Frustum`].
///
/// This system is used in [`CameraProjectionPlugin`](crate::CameraProjectionPlugin).
pub fn update_frusta(
    mut views: Query<
        (&GlobalTransform, &Projection, &mut Frustum),
        Or<(Changed<GlobalTransform>, Changed<Projection>)>,
    >,
) {
    for (transform, projection, mut frustum) in &mut views {
        *frustum = projection.compute_frustum(transform);
    }
}

fn visibility_propagate_system(
    changed: Query<
        (Entity, &Visibility, Option<&ChildOf>, Option<&Children>),
        (
            With<InheritedVisibility>,
            Or<(Changed<Visibility>, Changed<ChildOf>)>,
        ),
    >,
    mut visibility_query: Query<(&Visibility, &mut InheritedVisibility)>,
    children_query: Query<&Children, (With<Visibility>, With<InheritedVisibility>)>,
) {
    for (entity, visibility, child_of, children) in &changed {
        let is_visible = match visibility {
            Visibility::Visible => true,
            Visibility::Hidden => false,
            // fall back to true if no parent is found or parent lacks components
            Visibility::Inherited => child_of
                .and_then(|c| visibility_query.get(c.parent()).ok())
                .is_none_or(|(_, x)| x.get()),
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
/// As systems that check visibility judge entities visible, they remove them
/// from this set. Afterward, the `mark_newly_hidden_entities_invisible` system
/// runs and marks every mesh still remaining in this set as hidden.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct PreviousVisibleEntities(EntityHashSet);

/// Resets the view visibility of every entity.
/// Entities that are visible will be marked as such later this frame
/// by a [`VisibilitySystems::CheckVisibility`] system.
fn reset_view_visibility(
    mut query: Query<(Entity, &ViewVisibility)>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
) {
    previous_visible_entities.clear();

    query.iter_mut().for_each(|(entity, view_visibility)| {
        // Record the entities that were previously visible.
        if view_visibility.get() {
            previous_visible_entities.insert(entity);
        }
    });
}

/// System updating the visibility of entities each frame.
///
/// The system is part of the [`VisibilitySystems::CheckVisibility`] set. Each
/// frame, it updates the [`ViewVisibility`] of all entities, and for each view
/// also compute the [`VisibleEntities`] for that view.
///
/// To ensure that an entity is checked for visibility, make sure that it has a
/// [`VisibilityClass`] component and that that component is nonempty.
pub fn check_visibility(
    mut thread_queues: Local<Parallel<TypeIdMap<Vec<Entity>>>>,
    mut view_query: Query<(
        Entity,
        &mut VisibleEntities,
        &Frustum,
        Option<&RenderLayers>,
        &Camera,
        Has<NoCpuCulling>,
    )>,
    mut visible_aabb_query: Query<(
        Entity,
        &InheritedVisibility,
        &mut ViewVisibility,
        &VisibilityClass,
        Option<&RenderLayers>,
        Option<&Aabb>,
        &GlobalTransform,
        Has<NoFrustumCulling>,
        Has<VisibilityRange>,
    )>,
    visible_entity_ranges: Option<Res<VisibleEntityRanges>>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
) {
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
                    visibility_class,
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
                if !no_frustum_culling
                    && !no_cpu_culling
                    && let Some(model_aabb) = maybe_model_aabb
                {
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

                // Make sure we don't trigger changed notifications
                // unnecessarily by checking whether the flag is set before
                // setting it.
                if !**view_visibility {
                    view_visibility.set();
                }

                // Add the entity to the queue for all visibility classes the
                // entity is in.
                for visibility_class_id in visibility_class.iter() {
                    queue.entry(*visibility_class_id).or_default().push(entity);
                }
            },
        );

        visible_entities.clear_all();

        // Drain all the thread queues into the `visible_entities` list.
        for class_queues in thread_queues.iter_mut() {
            for (class, entities) in class_queues {
                let visible_entities_for_class = visible_entities.get_mut(*class);
                for entity in entities.drain(..) {
                    // As we mark entities as visible, we remove them from the
                    // `previous_visible_entities` list. At the end, all of the
                    // entities remaining in `previous_visible_entities` will be
                    // entities that were visible last frame but are no longer
                    // visible this frame.
                    previous_visible_entities.remove(&entity);

                    visible_entities_for_class.push(entity);
                }
            }
        }
    }
}

/// Marks any entities that weren't judged visible this frame as invisible.
///
/// As visibility-determining systems run, they remove entities that they judge
/// visible from [`PreviousVisibleEntities`]. At the end of visibility
/// determination, all entities that remain in [`PreviousVisibleEntities`] must
/// be invisible. This system goes through those entities and marks them newly
/// invisible (which sets the change flag for them).
fn mark_newly_hidden_entities_invisible(
    mut view_visibilities: Query<&mut ViewVisibility>,
    mut previous_visible_entities: ResMut<PreviousVisibleEntities>,
) {
    // Whatever previous visible entities are left are entities that were
    // visible last frame but just became invisible.
    for entity in previous_visible_entities.drain() {
        if let Ok(mut view_visibility) = view_visibilities.get_mut(entity) {
            *view_visibility = ViewVisibility::HIDDEN;
        }
    }
}

/// A generic component add hook that automatically adds the appropriate
/// [`VisibilityClass`] to an entity.
///
/// This can be handy when creating custom renderable components. To use this
/// hook, add it to your renderable component like this:
///
/// ```ignore
/// #[derive(Component)]
/// #[component(on_add = add_visibility_class::<MyComponent>)]
/// struct MyComponent {
///     ...
/// }
/// ```
pub fn add_visibility_class<C>(
    mut world: DeferredWorld<'_>,
    HookContext { entity, .. }: HookContext,
) where
    C: 'static,
{
    if let Some(mut visibility_class) = world.get_mut::<VisibilityClass>(entity) {
        visibility_class.push(TypeId::of::<C>());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_app::prelude::*;

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
    fn test_visibility_propagation_on_parent_change() {
        // Setup the world and schedule
        let mut app = App::new();

        app.add_systems(Update, visibility_propagate_system);

        // Create entities with visibility and hierarchy
        let parent1 = app.world_mut().spawn((Visibility::Hidden,)).id();
        let parent2 = app.world_mut().spawn((Visibility::Visible,)).id();
        let child1 = app.world_mut().spawn((Visibility::Inherited,)).id();
        let child2 = app.world_mut().spawn((Visibility::Inherited,)).id();

        // Build hierarchy
        app.world_mut()
            .entity_mut(parent1)
            .add_children(&[child1, child2]);

        // Run the system initially to set up visibility
        app.update();

        // Change parent visibility to Hidden
        app.world_mut()
            .entity_mut(parent2)
            .insert(Visibility::Visible);
        // Simulate a change in the parent component
        app.world_mut().entity_mut(child2).insert(ChildOf(parent2)); // example of changing parent

        // Run the system again to propagate changes
        app.update();

        let is_visible = |e: Entity| {
            app.world()
                .entity(e)
                .get::<InheritedVisibility>()
                .unwrap()
                .get()
        };

        // Retrieve and assert visibility

        assert!(
            !is_visible(child1),
            "Child1 should inherit visibility from parent"
        );

        assert!(
            is_visible(child2),
            "Child2 should inherit visibility from parent"
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
