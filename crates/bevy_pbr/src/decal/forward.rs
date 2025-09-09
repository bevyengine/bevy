use crate::{
    ExtendedMaterial, Material, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MaterialPlugin, StandardMaterial,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, Assets, Handle};
use bevy_ecs::{
    component::Component, lifecycle::HookContext, resource::Resource, world::DeferredWorld,
};
use bevy_math::{prelude::Rectangle, Quat, Vec2, Vec3};
use bevy_mesh::{Mesh, Mesh3d, MeshBuilder, MeshVertexBufferLayoutRef, Meshable};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    alpha::AlphaMode,
    render_asset::RenderAssets,
    render_resource::{
        AsBindGroup, AsBindGroupShaderType, CompareFunction, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    texture::GpuImage,
    RenderDebugFlags,
};
use bevy_shader::load_shader_library;

/// Plugin to render [`ForwardDecal`]s.
pub struct ForwardDecalPlugin;

impl Plugin for ForwardDecalPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "forward_decal.wgsl");

        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(
            Rectangle::from_size(Vec2::ONE)
                .mesh()
                .build()
                .rotated_by(Quat::from_rotation_arc(Vec3::Z, Vec3::Y))
                .with_generated_tangents()
                .unwrap(),
        );

        app.insert_resource(ForwardDecalMesh(mesh));

        app.add_plugins(MaterialPlugin::<ForwardDecalMaterial<StandardMaterial>> {
            prepass_enabled: false,
            shadows_enabled: false,
            debug_flags: RenderDebugFlags::default(),
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
/// * Any camera rendering a forward decal must have the [`bevy_core_pipeline::prepass::DepthPrepass`] component.
/// * Looking at forward decals at a steep angle can cause distortion. This can be mitigated by padding your decal's
///   texture with extra transparent pixels on the edges.
/// * On Wasm, requires using WebGPU and disabling `Msaa` on your camera.
#[derive(Component, Reflect)]
#[require(Mesh3d)]
#[component(on_add=forward_decal_set_mesh)]
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
#[uniform(200, ForwardDecalMaterialExtUniform)]
pub struct ForwardDecalMaterialExt {
    /// Controls the distance threshold for decal blending with surfaces.
    ///
    /// This parameter determines how far away a surface can be before the decal no longer blends
    /// with it and instead renders with full opacity.
    ///
    /// Lower values cause the decal to only blend with close surfaces, while higher values allow
    /// blending with more distant surfaces.
    ///
    /// Units are in meters.
    pub depth_fade_factor: f32,
}

#[derive(Clone, Default, ShaderType)]
pub struct ForwardDecalMaterialExtUniform {
    pub inv_depth_fade_factor: f32,
}

impl AsBindGroupShaderType<ForwardDecalMaterialExtUniform> for ForwardDecalMaterialExt {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<GpuImage>,
    ) -> ForwardDecalMaterialExtUniform {
        ForwardDecalMaterialExtUniform {
            inv_depth_fade_factor: 1.0 / self.depth_fade_factor.max(0.001),
        }
    }
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
            *label = format!("forward_decal_{label}").into();
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

#[derive(Resource)]
struct ForwardDecalMesh(Handle<Mesh>);

// Note: We need to use a hook here instead of required components since we cannot access resources
// with required components, and we can't otherwise get a handle to the asset from a required
// component constructor, since the constructor must be a function pointer, and we intentionally do
// not want to use `uuid_handle!`.
fn forward_decal_set_mesh(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let decal_mesh = world.resource::<ForwardDecalMesh>().0.clone();
    let mut entity = world.entity_mut(entity);
    let mut entity_mesh = entity.get_mut::<Mesh3d>().unwrap();
    // Only replace the mesh handle if the mesh handle is defaulted.
    if **entity_mesh == Handle::default() {
        entity_mesh.0 = decal_mesh;
    }
}
