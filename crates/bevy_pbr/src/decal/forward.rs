use crate::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Asset, Assets, Handle};
use bevy_ecs::component::Component;
use bevy_math::{prelude::Rectangle, Quat, Vec2, Vec3};
use bevy_reflect::TypePath;
use bevy_render::{
    alpha::AlphaMode,
    mesh::{Mesh, Mesh3d, MeshBuilder, MeshVertexBufferLayoutRef, Meshable},
    render_resource::{
        AsBindGroup, CompareFunction, RenderPipelineDescriptor, Shader,
        SpecializedMeshPipelineError,
    },
};

const FORWARD_DECAL_MESH_HANDLE: Handle<Mesh> = Handle::weak_from_u128(09376620402995522466);
const FORWARD_DECAL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(19376620402995522466);

/// TODO: Docs.
pub struct ForwardDecalPlugin;

impl Plugin for ForwardDecalPlugin {
    fn build(&self, app: &mut App) {
        let plane_mesh = Rectangle::from_size(Vec2::ONE)
            .mesh()
            .build()
            .rotated_by(Quat::from_rotation_arc(Vec3::Z, Vec3::Y))
            .with_generated_tangents()
            .unwrap();

        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .insert(FORWARD_DECAL_MESH_HANDLE.id(), plane_mesh);

        load_internal_asset!(
            app,
            FORWARD_DECAL_SHADER_HANDLE,
            "forward_decal.wgsl",
            Shader::from_wgsl
        );
    }
}

/// TODO: Docs.
#[derive(Component)]
#[require(Mesh3d(|| Mesh3d(FORWARD_DECAL_MESH_HANDLE)))]
pub struct ForwardDecal;

/// TODO: Docs.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub struct ForwardDecalMaterial {
    #[uniform(200)]
    pub depth_fade_factor: f32,
}

impl MaterialExtension for ForwardDecalMaterial {
    fn alpha_mode() -> Option<AlphaMode> {
        Some(AlphaMode::Blend)
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .depth_stencil
            .as_mut()
            .expect("TODO")
            .depth_compare = CompareFunction::Always;

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

impl Default for ForwardDecalMaterial {
    fn default() -> Self {
        Self {
            depth_fade_factor: 8.0,
        }
    }
}
