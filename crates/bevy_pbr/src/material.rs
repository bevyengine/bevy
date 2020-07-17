use bevy_asset::{self, Handle};
use bevy_render::{color::Color, renderer::RenderResources, shader::ShaderDefs, texture::Texture};

#[derive(RenderResources, ShaderDefs)]
pub struct StandardMaterial {
    pub albedo: Color,
    #[shader_def]
    pub albedo_texture: Option<Handle<Texture>>,
    #[render_resources(ignore)]
    #[shader_def]
    pub shaded: bool,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            albedo: Color::rgb(1.0, 1.0, 1.0),
            albedo_texture: None,
            shaded: true,
        }
    }
}
