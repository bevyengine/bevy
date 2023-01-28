use crate::MaterialPlugin;
use crate::{Material, MaterialPipeline};
use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_reflect::TypeUuid;
use bevy_render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef};
use bevy_render::{
    mesh::MeshVertexBufferLayout,
    render_resource::{Shader, SpecializedMeshPipelineError},
};

pub const DEBUG_MESH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 6072173229347232709);

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum DebugMeshKey {
    WorldPosition,
    WorldNormal,
    UVs,
    WorldTangent,
}

impl From<&DebugMeshMaterial> for DebugMeshKey {
    fn from(material: &DebugMeshMaterial) -> Self {
        material.variant
    }
}

#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "7638ab74-9e99-11ed-bfc6-c3b5616dd440"]
#[bind_group_data(DebugMeshKey)]
pub struct DebugMeshMaterial {
    pub variant: DebugMeshKey,
}

impl Material for DebugMeshMaterial {
    fn fragment_shader() -> ShaderRef {
        DEBUG_MESH_SHADER_HANDLE.typed().into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: crate::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor
            .fragment
            .as_mut()
            .unwrap()
            .shader_defs
            .push(match key.bind_group_data {
                DebugMeshKey::WorldPosition => "DEBUG_WORLD_POSITION".into(),
                DebugMeshKey::WorldNormal => "DEBUG_WORLD_NORMAL".into(),
                DebugMeshKey::UVs => "DEBUG_UVS".into(),
                DebugMeshKey::WorldTangent => "DEBUG_WORLD_TANGENT".into(),
            });

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DebugMeshPlugin;

impl Plugin for DebugMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            DEBUG_MESH_SHADER_HANDLE,
            "render/debug_mesh.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(MaterialPlugin::<DebugMeshMaterial>::default());
    }
}
