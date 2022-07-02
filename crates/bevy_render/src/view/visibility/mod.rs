mod render_layers;

use bevy_math::Vec3A;
pub use render_layers::*;

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, HierarchySystem, Parent};
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

/// User indication of whether an entity is visible
#[derive(Component, Clone, Reflect, Debug, Eq, PartialEq)]
#[reflect(Component, Default)]
pub struct Visibility {
    self_visible: bool,
    inherited: bool,
}

impl Visibility {
    /// Checks if the entity is visible or not.
    ///
    /// This value can be affected by the entity's local visibility
    /// or it inherited from it's ancestors in the hierarchy. An entity
    /// is only visible in the hierarchy if and only if all of it's
    /// ancestors are visible. Setting an entity to be invisible will
    /// hide all of it's children and descendants.
    ///
    /// If the systems labeled [`VisibilitySystems::VisibilityPropagate`]
    /// have not yet run, this value may be out of date with the state of
    /// the hierarchy.
    pub fn is_visible(&self) -> bool {
        self.self_visible && self.inherited
    }

    /// Checks the local visibility state of the entity.
    ///
    /// Unlike [`is_visible`](Self::is_visible), this value is always up to date.
    pub fn is_self_visible(&self) -> bool {
        self.self_visible
    }

    /// Sets whether the entity is visible or not. If set to false,
    /// all descendants will be marked as invisible.
    pub fn set_visible(&mut self, visible: bool) {
        self.self_visible = visible;
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            self_visible: true,
            inherited: true,
        }
    }
}

/// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Component)]
pub struct ComputedVisibility {
    pub is_visible: bool,
}

impl Default for ComputedVisibility {
    fn default() -> Self {
        Self { is_visible: true }
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
            visibility_propagate_system
                .label(VisibilityPropagate)
                .after(HierarchySystem::ParentUpdate),
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
    for (entity, mesh_handle) in without_aabb.iter() {
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
    for (transform, projection, mut frustum) in views.iter_mut() {
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
    mut root_query: Query<(Option<&Children>, &mut Visibility, Entity), Without<Parent>>,
    mut visibility_query: Query<(&mut Visibility, &Parent)>,
    children_query: Query<&Children, (With<Parent>, With<Visibility>)>,
) {
    for (children, mut visibility, entity) in root_query.iter_mut() {
        // Avoid triggering change detection if nothing has changed.
        if !visibility.inherited {
            visibility.inherited = true;
        }
        if let Some(children) = children {
            let is_visible = visibility.is_visible();
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
    visibility_query: &mut Query<(&mut Visibility, &Parent)>,
    children_query: &Query<&Children, (With<Parent>, With<Visibility>)>,
    entity: Entity,
    expected_parent: Entity,
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    let is_visible = {
        let (mut visibility, child_parent) = visibility_query.get_mut(entity).map_err(drop)?;
        // Note that for parallelising, this check cannot occur here, since there is an `&mut GlobalTransform` (in global_transform)
        assert_eq!(
            child_parent.0, expected_parent,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        // Avoid triggering change detection if nothing has changed.
        if visibility.inherited != parent_visible {
            visibility.inherited = parent_visible;
        }
        visibility.is_visible()
    };

    for child in children_query.get(entity).map_err(drop)?.iter() {
        let _ = propagate_recursive(is_visible, visibility_query, children_query, *child, entity);
    }
    Ok(())
}

/// System updating the visibility of entities each frame.
///
/// The system is labelled with [`VisibilitySystems::CheckVisibility`]. Each frame, it updates the
/// [`ComputedVisibility`] of all entities, and for each view also compute the [`VisibleEntities`]
/// for that view.
pub fn check_visibility(
    mut thread_queues: Local<ThreadLocal<Cell<Vec<Entity>>>>,
    mut view_query: Query<(&mut VisibleEntities, &Frustum, Option<&RenderLayers>), With<Camera>>,
    mut visible_entity_query: ParamSet<(
        Query<&mut ComputedVisibility>,
        Query<(
            Entity,
            &Visibility,
            &mut ComputedVisibility,
            Option<&RenderLayers>,
            Option<&Aabb>,
            Option<&NoFrustumCulling>,
            Option<&GlobalTransform>,
        )>,
    )>,
) {
    // Reset the computed visibility to false
    for mut computed_visibility in visible_entity_query.p0().iter_mut() {
        computed_visibility.is_visible = false;
    }

    for (mut visible_entities, frustum, maybe_view_mask) in view_query.iter_mut() {
        let view_mask = maybe_view_mask.copied().unwrap_or_default();
        visible_entities.entities.clear();
        visible_entity_query.p1().par_for_each_mut(
            1024,
            |(
                entity,
                visibility,
                mut computed_visibility,
                maybe_entity_mask,
                maybe_aabb,
                maybe_no_frustum_culling,
                maybe_transform,
            )| {
                if !visibility.is_visible() {
                    return;
                }

                let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
                if !view_mask.intersects(&entity_mask) {
                    return;
                }

                // If we have an aabb and transform, do frustum culling
                if let (Some(model_aabb), None, Some(transform)) =
                    (maybe_aabb, maybe_no_frustum_culling, maybe_transform)
                {
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

                computed_visibility.is_visible = true;
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

    use bevy_hierarchy::{parent_update_system, BuildWorldChildren, Children, Parent};

    #[test]
    fn did_propagate() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system);
        update_stage.add_system(visibility_propagate_system.after(parent_update_system));

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Root entity
        world.spawn().insert(Visibility::default());

        let mut children = Vec::new();
        world
            .spawn()
            .insert(Visibility {
                self_visible: false,
                inherited: false,
            })
            .with_children(|parent| {
                children.push(parent.spawn().insert(Visibility::default()).id());
                children.push(parent.spawn().insert(Visibility::default()).id());
            });
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<Visibility>(children[0]).unwrap(),
            Visibility {
                self_visible: true,
                inherited: false,
            }
        );

        assert_eq!(
            *world.get::<Visibility>(children[1]).unwrap(),
            Visibility {
                self_visible: true,
                inherited: false,
            }
        );
    }

    #[test]
    fn correct_visibility_when_no_children() {
        let mut app = App::new();

        app.add_system(parent_update_system);
        app.add_system(visibility_propagate_system.after(parent_update_system));

        let parent = app
            .world
            .spawn()
            .insert(Visibility {
                self_visible: false,
                inherited: false,
            })
            .insert(GlobalTransform::default())
            .id();

        let child = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), Parent(parent)))
            .id();

        let grandchild = app
            .world
            .spawn()
            .insert_bundle((Visibility::default(), Parent(child)))
            .id();

        app.update();

        // check the `Children` structure is spawned
        assert_eq!(&**app.world.get::<Children>(parent).unwrap(), &[child]);
        assert_eq!(&**app.world.get::<Children>(child).unwrap(), &[grandchild]);
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to `Commands` delay
        app.update();

        let mut state = app.world.query::<&Visibility>();
        for visibility in state.iter(&app.world) {
            assert!(!visibility.is_visible());
        }
    }

    #[test]
    #[should_panic]
    fn panic_when_hierarchy_cycle() {
        let mut world = World::default();
        // This test is run on a single thread in order to avoid breaking the global task pool by panicking
        // This fixes the flaky tests reported in https://github.com/bevyengine/bevy/issues/4996
        let mut update_stage = SystemStage::single_threaded();

        update_stage.add_system(parent_update_system);
        update_stage.add_system(visibility_propagate_system.after(parent_update_system));

        let child = world.spawn().insert(Visibility::default()).id();

        let grandchild = world
            .spawn()
            .insert_bundle((Visibility::default(), Parent(child)))
            .id();
        world
            .spawn()
            .insert_bundle((Visibility::default(), Children::with(&[child])));
        world.entity_mut(child).insert(Parent(grandchild));

        update_stage.run(&mut world);
    }
}
