use bevy_asset::{weak_handle, Asset, Handle};
use bevy_color::LinearRgba;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::{
        AsBindGroup, PolygonMode, RenderPipelineDescriptor, Shader, ShaderRef,
        SpecializedMeshPipelineError,
    },
};

use crate::{material::rendering::Material2dKey, prelude::Material2d};

pub const WIREFRAME_2D_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("3d8a3853-2927-4de2-9dc7-3971e7e40970");

#[derive(Default, AsBindGroup, Debug, Clone, Asset, Reflect)]
#[reflect(Clone, Default)]
pub struct Wireframe2dMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for Wireframe2dMaterial {
    fn fragment_shader() -> ShaderRef {
        WIREFRAME_2D_SHADER_HANDLE.into()
    }

    fn depth_bias(&self) -> f32 {
        1.0
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.polygon_mode = PolygonMode::Line;
        Ok(())
    }
}
