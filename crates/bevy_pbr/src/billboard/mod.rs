//! Meshes that always face the camera.

use crate::{
    shader_ref, ExtendedMaterial, MaterialExtension, MaterialExtensionKey,
    MaterialExtensionPipeline, MaterialPlugin, StandardMaterial,
};
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, embedded_path, Asset};
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_reflect::Reflect;
use bevy_render::render_resource::{
    AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_shader::{ShaderDefVal, ShaderRef};

/// Adds support for [`BillboardMaterial`].
pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "billboard.wesl");
        app.add_plugins(MaterialPlugin::<BillboardMaterial>::default());
    }
}

/// A [`StandardMaterial`] that rotates its mesh to face the camera.
///
/// Currently, there are few limitations to billboards:
/// - Billboards receive shadows but do not cast them (you can disable
///   shadows with [`NotShadowReceiver`](bevy_light::NotShadowReceiver)).
/// - Skinned and morphed meshes are not supported.
/// - The mesh should lie in its local XY plane, and its
///   [`Aabb`](bevy_camera::primitives::Aabb) must enclose the mesh at every
///   rotation for frustum culling to be correct.
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
