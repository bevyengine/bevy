use crate::{
    math::Vec4,
    render::render_graph_2::{
        uniform::{AsUniforms, GetBytes, UniformInfo},
        BindType, UniformPropertyType, ShaderDefSuffixProvider
    },
};

use bevy_derive::Uniforms;

#[derive(Uniforms)]
pub struct StandardMaterial {
    pub albedo: Vec4,
    #[uniform(ignore,shader_def)]
    pub everything_is_red: bool,
}