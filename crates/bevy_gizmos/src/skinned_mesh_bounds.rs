//! A module adding debug visualization of [`SkinnedMeshBounds`]s.

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    query::Without,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
};
use bevy_math::Affine3A;
use bevy_mesh::skinning::{
    SkinnedMesh, SkinnedMeshBounds, SkinnedMeshBoundsAsset, SkinnedMeshInverseBindposes,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystems};

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of [`SkinnedMeshBounds`]s for debugging.
pub struct SkinnedMeshBoundsGizmoPlugin;

impl Plugin for SkinnedMeshBoundsGizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_gizmo_group::<SkinnedMeshBoundsGizmoConfigGroup>()
            .add_systems(
                PostUpdate,
                (
                    draw_skinned_mesh_bounds,
                    draw_all_skinned_mesh_bounds.run_if(|config: Res<GizmoConfigStore>| {
                        config
                            .config::<SkinnedMeshBoundsGizmoConfigGroup>()
                            .1
                            .draw_all
                    }),
                )
                    .after(TransformSystems::Propagate),
            );
    }
}
/// The [`GizmoConfigGroup`] used for debug visualizations of [`SkinnedMeshBounds`] components on entities.
#[derive(Clone, Reflect, GizmoConfigGroup)]
#[reflect(Clone, Default)]
pub struct SkinnedMeshBoundsGizmoConfigGroup {
    /// When set to `true`, draws all the bounds that contribute to skinned mesh
    /// bounds.
    ///
    /// To draw a specific entity's skinned mesh bounds, you can add the [`ShowSkinnedMeshBoundsGizmo`] component.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// The default color for skinned mesh bounds gizmos.
    pub default_color: Color,
}

impl Default for SkinnedMeshBoundsGizmoConfigGroup {
    fn default() -> Self {
        Self {
            draw_all: false,
            default_color: Color::WHITE,
        }
    }
}

/// Add this [`Component`] to an entity to draw its [`SkinnedMeshBounds`] component.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default, Debug)]
pub struct ShowSkinnedMeshBoundsGizmo {
    /// The color of the bounds.
    ///
    /// The default color from the [`SkinnedMeshBoundsGizmoConfigGroup`] config is used if `None`,
    pub color: Option<Color>,
}

fn draw(
    color: Color,
    mesh: &SkinnedMesh,
    bounds: &SkinnedMeshBounds,
    joints: &Query<&GlobalTransform>,
    bounds_assets: &Res<Assets<SkinnedMeshBoundsAsset>>,
    inverse_bindposes_assets: &Res<Assets<SkinnedMeshInverseBindposes>>,
    gizmos: &mut Gizmos<SkinnedMeshBoundsGizmoConfigGroup>,
) {
    if let Some(bounds_asset) = bounds_assets.get(bounds)
        && let Some(inverse_bindposes_asset) = inverse_bindposes_assets.get(&mesh.inverse_bindposes)
    {
        for (&joint_index, &joint_aabb) in bounds_asset
            .aabb_index_to_joint_index
            .iter()
            .zip(bounds_asset.aabbs.iter())
        {
            let joint_index = joint_index as usize;

            if let Some(&joint_entity) = mesh.joints.get(joint_index)
                && let Ok(&world_from_joint) = joints.get(joint_entity)
                && let Some(&joint_from_mesh) = inverse_bindposes_asset.get(joint_index)
            {
                let world_from_mesh =
                    world_from_joint.affine() * Affine3A::from_mat4(joint_from_mesh);

                gizmos.aabb_3d(joint_aabb, world_from_mesh, color);
            }
        }
    }
}

fn draw_skinned_mesh_bounds(
    meshes: Query<(
        &SkinnedMesh,
        &SkinnedMeshBounds,
        &ShowSkinnedMeshBoundsGizmo,
    )>,
    joints: Query<&GlobalTransform>,
    bounds_assets: Res<Assets<SkinnedMeshBoundsAsset>>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut gizmos: Gizmos<SkinnedMeshBoundsGizmoConfigGroup>,
) {
    for (mesh_bounds, mesh, gizmo) in meshes {
        let color = gizmo.color.unwrap_or(gizmos.config_ext.default_color);

        draw(
            color,
            mesh_bounds,
            mesh,
            &joints,
            &bounds_assets,
            &inverse_bindposes_assets,
            &mut gizmos,
        );
    }
}

fn draw_all_skinned_mesh_bounds(
    meshes: Query<(&SkinnedMesh, &SkinnedMeshBounds), Without<ShowSkinnedMeshBoundsGizmo>>,
    joints: Query<&GlobalTransform>,
    bounds_assets: Res<Assets<SkinnedMeshBoundsAsset>>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut gizmos: Gizmos<SkinnedMeshBoundsGizmoConfigGroup>,
) {
    for (mesh_bounds, mesh) in meshes {
        draw(
            gizmos.config_ext.default_color,
            mesh_bounds,
            mesh,
            &joints,
            &bounds_assets,
            &inverse_bindposes_assets,
            &mut gizmos,
        );
    }
}
