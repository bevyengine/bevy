use crate::{
    pipeline::{
        CullMode, FrontFace, PipelineDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    },
    shader::{Shader, ShaderStage, ShaderStages},
};
use bevy_asset::Assets;

pub(crate) fn build_wireframe_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        name: Some("wireframe".into()),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            polygon_mode: PolygonMode::Line,
        },
        ..PipelineDescriptor::default_config(ShaderStages {
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
