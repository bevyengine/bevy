use crate::{
    math,
    math::Vec4,
    render::render_graph_2::{
        uniform::{AsUniforms, GetBytes, UniformInfo},
        BindType, ShaderDefSuffixProvider, UniformPropertyType,
    },
};

use bevy_derive::Uniforms;

#[derive(Uniforms)]
pub struct StandardMaterial {
    pub albedo: Vec4,
    #[uniform(ignore, shader_def)]
    pub everything_is_red: bool,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            albedo: math::vec4(0.3, 0.3, 0.3, 1.0),
            everything_is_red: false,
        }
    }
}
