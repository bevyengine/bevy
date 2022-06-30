mod render_layers;

use bevy_math::Vec3A;
pub use render_layers::*;

use bevy_app::{CoreStage, Plugin};
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
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
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Component, Default)]
pub struct Visibility {
    pub is_visible: bool,
}

impl Default for Visibility {
    fn default() -> Self {
        Self { is_visible: true }
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
            check_visibility
                .label(CheckVisibility)
                .after(CalculateBounds)
                .after(UpdateOrthographicFrusta)
                .after(UpdatePerspectiveFrusta)
                .after(UpdateProjectionFrusta)
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
                if !visibility.is_visible {
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
