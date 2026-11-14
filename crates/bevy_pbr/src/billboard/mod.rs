//! Meshes that always face the camera.

use crate::{
    shader_ref, ExtendedMaterial, MaterialExtension, MaterialExtensionKey,
    MaterialExtensionPipeline, MaterialPlugin, MeshMaterial3d, StandardMaterial,
};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{embedded_asset, embedded_path, Asset, Assets};
use bevy_camera::{
    primitives::{Aabb, MeshAabb},
    visibility::{calculate_bounds, NoAutoAabb, NoFrustumCulling, VisibilitySystems},
};
use bevy_ecs::prelude::*;
use bevy_math::{Vec3, Vec3A};
use bevy_mesh::{Mesh, Mesh3d, MeshVertexBufferLayoutRef};
use bevy_reflect::Reflect;
use bevy_render::render_resource::{
    AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_shader::{ShaderDefVal, ShaderRef};
use bevy_transform::components::GlobalTransform;

/// Adds support for [`BillboardMaterial`].
pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "billboard.wesl");
        app.add_plugins(MaterialPlugin::<BillboardMaterial>::default())
            .add_systems(
                PostUpdate,
                update_billboard_bounds
                    .after(calculate_bounds)
                    .in_set(VisibilitySystems::CalculateBounds),
            );
    }
}

/// A [`StandardMaterial`] that rotates its mesh to face the camera.
///
/// Currently, there are few limitations to billboards:
/// - Billboards receive shadows but do not cast them (you can disable
///   shadows with [`NotShadowReceiver`](bevy_light::NotShadowReceiver)).
/// - Skinned and morphed meshes are not supported.
/// - The mesh should lie in its local XY plane.
///
/// The [`Aabb`] of a billboard is replaced with one that encloses its mesh at
/// every rotation, for frustum culling to be correct. Add [`NoAutoAabb`] to
/// provide your own instead.
///
/// A negative `Transform` scale does not mirror a billboard. Use
/// [`StandardMaterial::flip`] on the base material instead. This only flips the
/// texture coordinates, so normal maps are not corrected for the mirroring.
///
/// The rotation only happens in the vertex shader, so anything on the CPU sees
/// the unrotated mesh. In particular, `MeshRayCast` and `MeshPickingPlugin` test
/// against the mesh in its local XY plane, and miss a visible billboard once the
/// camera has moved around it. Picking could be improved by a backend that
/// applies the same camera-facing rotation before testing a billboard's mesh.
pub type BillboardMaterial = ExtendedMaterial<StandardMaterial, BillboardExtension>;

/// The [`MaterialExtension`] for [`BillboardMaterial`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bindless(index_table(range(50..51), binding(100)))]
#[bind_group_data(BillboardExtensionKey)]
pub struct BillboardExtension {
    /// Keeps the billboard upright, rotating it only around the world Y axis.
    pub lock_y: bool,
    /// Whether the billboard can be hidden by geometry in front of it.
    ///
    /// When this is `false`, the main pass of the billboard does not write depth.
    /// Prepasses still run with the usual depth test, so they only see the parts
    /// of it that are not hidden. Use it with
    /// [`AlphaMode::Mask`] or [`AlphaMode::Blend`], which draw after opaque
    /// geometry: an [`AlphaMode::Opaque`] billboard can still be covered by
    /// whatever draws after it.
    ///
    /// This has no effect on a deferred billboard. If deferred rendering is the
    /// default, set [`StandardMaterial::opaque_render_method`] to
    /// [`OpaqueRendererMethod::Forward`] on the base material.
    ///
    /// [`AlphaMode::Mask`]: bevy_material::AlphaMode::Mask
    /// [`AlphaMode::Blend`]: bevy_material::AlphaMode::Blend
    /// [`AlphaMode::Opaque`]: bevy_material::AlphaMode::Opaque
    /// [`OpaqueRendererMethod::Forward`]: bevy_material::OpaqueRendererMethod::Forward
    pub depth_test: bool,
}

impl Default for BillboardExtension {
    fn default() -> Self {
        Self {
            lock_y: false,
            depth_test: true,
        }
    }
}

/// The pipeline key of a [`BillboardExtension`].
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct BillboardExtensionKey {
    lock_y: bool,
    depth_test: bool,
}

impl From<&BillboardExtension> for BillboardExtensionKey {
    fn from(extension: &BillboardExtension) -> Self {
        Self {
            lock_y: extension.lock_y,
            depth_test: extension.depth_test,
        }
    }
}

impl MaterialExtension for BillboardExtension {
    fn enable_oit() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn vertex_shader() -> ShaderRef {
        shader_ref(embedded_path!("billboard.wesl"))
    }

    fn prepass_vertex_shader() -> ShaderRef {
        Self::vertex_shader()
    }

    fn deferred_vertex_shader() -> ShaderRef {
        Self::vertex_shader()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let shader_defs = &mut descriptor.vertex.shader_defs;
        if key.bind_group_data.lock_y {
            shader_defs.push("BILLBOARD_LOCK_Y".into());
        }

        let is_prepass = shader_defs.contains(&ShaderDefVal::from("PREPASS_PIPELINE"));
        if !key.bind_group_data.depth_test
            && !is_prepass
            && let Some(depth_stencil) = &mut descriptor.depth_stencil
        {
            depth_stencil.depth_compare = Some(CompareFunction::Always);
            depth_stencil.depth_write_enabled = Some(false);
        }

        Ok(())
    }
}

/// Replaces the [`Aabb`] of billboards, which [`calculate_bounds`] fits to the
/// unrotated mesh, with one that encloses the mesh at every rotation.
fn update_billboard_bounds(
    meshes: Res<Assets<Mesh>>,
    mut billboards: Query<
        (&Mesh3d, &GlobalTransform, &mut Aabb),
        (
            With<MeshMaterial3d<BillboardMaterial>>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
            Or<(
                Changed<Aabb>,
                Changed<GlobalTransform>,
                Changed<MeshMaterial3d<BillboardMaterial>>,
            )>,
        ),
    >,
) {
    billboards
        .par_iter_mut()
        .for_each(|(mesh, transform, mut aabb)| {
            let matrix = transform.affine().matrix3;
            let scale = Vec3::new(
                matrix.x_axis.length(),
                matrix.y_axis.length(),
                matrix.z_axis.length(),
            );
            if let Some(mesh_aabb) = meshes.get(mesh).and_then(MeshAabb::get_aabb)
                && let Some(billboard_aabb) = billboard_aabb(&mesh_aabb, scale)
            {
                aabb.set_if_neq(billboard_aabb);
            }
        });
}

/// Returns the local [`Aabb`] of a billboard with the given mesh bounds and
/// world scale, or [`None`] if it's scaled to nothing. The bounds are a cube
/// that still encloses the billboard along the least scaled axis.
fn billboard_aabb(mesh_aabb: &Aabb, scale: Vec3) -> Option<Aabb> {
    let farthest_corner = Vec3::from(mesh_aabb.center.abs() + mesh_aabb.half_extents);
    let min_scale = scale.min_element();
    (min_scale > 0.0).then(|| Aabb {
        center: Vec3A::ZERO,
        half_extents: Vec3A::splat((farthest_corner * scale).length() / min_scale),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billboard_aabb_encloses_every_rotation() {
        let mesh_aabb = Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::new(0.5, 0.5, 0.0),
        };
        let half_extents = |scale| Some(billboard_aabb(&mesh_aabb, scale)?.half_extents);

        assert_eq!(half_extents(Vec3::ONE), Some(Vec3A::splat(0.5_f32.sqrt())));
        assert_eq!(
            half_extents(Vec3::new(2.0, 1.0, 1.0)),
            Some(Vec3A::splat(1.25_f32.sqrt()))
        );
        assert_eq!(half_extents(Vec3::ZERO), None);
    }
}
