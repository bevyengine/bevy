use crate::{
    pipeline::{FrontFace, RenderPipelineDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology},
    shader::{Shader, ShaderStage, ShaderStages},
};
use bevy_asset::Assets;

pub(crate) fn build_wireframe_pipeline(shaders: &mut Assets<Shader>) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        name: Some("wireframe".into()),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: PolygonMode::Line,
            clamp_depth: false,
            conservative: false,
        },
        ..RenderPipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("wireframe.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("wireframe.frag"),
            ))),
        })
    }
}
