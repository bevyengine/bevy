use crate::{
    ExtendedMaterial, Material, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MaterialPlugin, StandardMaterial,
};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Asset, Assets, Handle};
use bevy_ecs::component::{require, Component};
use bevy_math::{prelude::Rectangle, Quat, Vec2, Vec3};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    alpha::AlphaMode,
    mesh::{Mesh, Mesh3d, MeshBuilder, MeshVertexBufferLayoutRef, Meshable},
    render_resource::{
        AsBindGroup, CompareFunction, RenderPipelineDescriptor, Shader,
        SpecializedMeshPipelineError,
    },
};

const FORWARD_DECAL_MESH_HANDLE: Handle<Mesh> = Handle::weak_from_u128(19376620402995522466);
const FORWARD_DECAL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(29376620402995522466);

/// Plugin to render [`ForwardDecal`]s.
pub struct ForwardDecalPlugin;

impl Plugin for ForwardDecalPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            FORWARD_DECAL_SHADER_HANDLE,
            "forward_decal.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ForwardDecal>();

        app.world_mut().resource_mut::<Assets<Mesh>>().insert(
            FORWARD_DECAL_MESH_HANDLE.id(),
            Rectangle::from_size(Vec2::ONE)
                .mesh()
                .build()
                .rotated_by(Quat::from_rotation_arc(Vec3::Z, Vec3::Y))
                .with_generated_tangents()
                .unwrap(),
        );

        app.add_plugins(MaterialPlugin::<ForwardDecalMaterial<StandardMaterial>> {
            prepass_enabled: false,
            shadows_enabled: false,
            ..Default::default()
        });
    }
}

/// A decal that renders via a 1x1 transparent quad mesh, smoothly alpha-blending with the underlying
/// geometry towards the edges.
///
/// Because forward decals are meshes, you can use arbitrary materials to control their appearance.
///
/// # Usage Notes
///
/// * Spawn this component on an entity with a [`crate::MeshMaterial3d`] component holding a [`ForwardDecalMaterial`].
/// * Any camera rendering a forward decal must have the [`bevy_core_pipeline::DepthPrepass`] component.
/// * Looking at forward decals at a steep angle can cause distortion. This can be mitigated by padding your decal's
///   texture with extra transparent pixels on the edges.
#[derive(Component, Reflect)]
#[require(Mesh3d(|| Mesh3d(FORWARD_DECAL_MESH_HANDLE)))]
pub struct ForwardDecal;

/// Type alias for an extended material with a [`ForwardDecalMaterialExt`] extension.
///
/// Make sure to register the [`MaterialPlugin`] for this material in your app setup.
///
/// [`StandardMaterial`] comes with out of the box support for forward decals.
#[expect(type_alias_bounds, reason = "Type alias generics not yet stable")]
pub type ForwardDecalMaterial<B: Material> = ExtendedMaterial<B, ForwardDecalMaterialExt>;

/// Material extension for a [`ForwardDecal`].
///
/// In addition to wrapping your material type with this extension, your shader must use
/// the `bevy_pbr::decal::forward::get_forward_decal_info` function.
///
/// The `FORWARD_DECAL` shader define will be made available to your shader so that you can gate
/// the forward decal code behind an ifdef.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub struct ForwardDecalMaterialExt {
    /// Controls how far away a surface must be before the decal will stop blending with it, and instead render as opaque.
    ///
    /// Decreasing this value will cause the decal to blend only to surfaces closer to it.
    ///
    /// Units are in meters.
    #[uniform(200)]
    pub depth_fade_factor: f32,
}

impl MaterialExtension for ForwardDecalMaterialExt {
    fn alpha_mode() -> Option<AlphaMode> {
        Some(AlphaMode::Blend)
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.depth_stencil.as_mut().unwrap().depth_compare = CompareFunction::Always;

        descriptor.vertex.shader_defs.push("FORWARD_DECAL".into());

        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader_defs.push("FORWARD_DECAL".into());
        }

        if let Some(label) = &mut descriptor.label {
            *label = format!("forward_decal_{}", label).into();
        }

        Ok(())
    }
}

impl Default for ForwardDecalMaterialExt {
    fn default() -> Self {
        Self {
            depth_fade_factor: 8.0,
        }
    }
}
