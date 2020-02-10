use crate::{
    math::Vec4,
    render::render_graph_2::{
        uniform::{AsUniforms, GetBytes, UniformInfo},
        BindType, UniformPropertyType,
    },
};

use bevy_derive::Uniforms;

#[derive(Uniforms)]
pub struct StandardMaterial {
    pub albedo: Vec4,
    // #[uniform(ignore,shader_def="Hi")]
    // pub enable_thing: bool,
}