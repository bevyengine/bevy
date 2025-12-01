//! A module adding debug visualization of [`DynamicSkinnedMeshBounds`].

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_camera::visibility::DynamicSkinnedMeshBounds;
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    query::{With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
};
use bevy_math::Affine3A;
use bevy_mesh::{
    mark_3d_meshes_as_changed_if_their_assets_changed,
    skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    Mesh, Mesh3d,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::{components::GlobalTransform, TransformSystems};

use crate::{
    config::{GizmoConfigGroup, GizmoConfigStore},
    gizmos::Gizmos,
    AppGizmoBuilder,
};

/// A [`Plugin`] that provides visualization of entities with [`DynamicSkinnedMeshBounds`].
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
                    .after(TransformSystems::Propagate)
                    .ambiguous_with(mark_3d_meshes_as_changed_if_their_assets_changed),
            );
    }
}
/// The [`GizmoConfigGroup`] used for debug visualizations of entities with [`DynamicSkinnedMeshBounds`]
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

/// Add this [`Component`] to an entity to draw its [`DynamicSkinnedMeshBounds`] component.
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
    mesh: &Mesh3d,
    mesh_assets: &Res<Assets<Mesh>>,
    skinned_mesh: &SkinnedMesh,
    joint_entities: &Query<&GlobalTransform>,
    inverse_bindposes_assets: &Res<Assets<SkinnedMeshInverseBindposes>>,
    gizmos: &mut Gizmos<SkinnedMeshBoundsGizmoConfigGroup>,
) {
    if let Some(mesh_asset) = mesh_assets.get(mesh)
        && let Some(bounds) = mesh_asset.skinned_mesh_bounds()
        && let Some(inverse_bindposes_asset) =
            inverse_bindposes_assets.get(&skinned_mesh.inverse_bindposes)
    {
        for (&joint_index, &joint_aabb) in bounds.iter() {
            let joint_index = joint_index.0 as usize;

            if let Some(&joint_entity) = skinned_mesh.joints.get(joint_index)
                && let Ok(&world_from_joint) = joint_entities.get(joint_entity)
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
    mesh_entities: Query<
        (&Mesh3d, &SkinnedMesh, &ShowSkinnedMeshBoundsGizmo),
        With<DynamicSkinnedMeshBounds>,
    >,
    joint_entities: Query<&GlobalTransform>,
    mesh_assets: Res<Assets<Mesh>>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut gizmos: Gizmos<SkinnedMeshBoundsGizmoConfigGroup>,
) {
    for (mesh, skinned_mesh, gizmo) in mesh_entities {
        let color = gizmo.color.unwrap_or(gizmos.config_ext.default_color);

        draw(
            color,
            mesh,
            &mesh_assets,
            skinned_mesh,
            &joint_entities,
            &inverse_bindposes_assets,
            &mut gizmos,
        );
    }
}

fn draw_all_skinned_mesh_bounds(
    mesh_entities: Query<
        (&Mesh3d, &SkinnedMesh),
        (
            With<DynamicSkinnedMeshBounds>,
            Without<ShowSkinnedMeshBoundsGizmo>,
        ),
    >,
    joint_entities: Query<&GlobalTransform>,
    mesh_assets: Res<Assets<Mesh>>,
    inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut gizmos: Gizmos<SkinnedMeshBoundsGizmoConfigGroup>,
) {
    for (mesh, skinned_mesh) in mesh_entities {
        draw(
            gizmos.config_ext.default_color,
            mesh,
            &mesh_assets,
            skinned_mesh,
            &joint_entities,
            &inverse_bindposes_assets,
            &mut gizmos,
        );
    }
}
