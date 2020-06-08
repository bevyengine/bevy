use bevy_asset::{self, Handle};
use bevy_render::{render_resource::RenderResources, shader::ShaderDefs, texture::Texture, Color};

#[derive(RenderResources, ShaderDefs)]
pub struct StandardMaterial {
    pub albedo: Color,
    #[shader_def]
    pub albedo_texture: Option<Handle<Texture>>,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            albedo: Color::rgb(1.0, 1.0, 1.0),
            albedo_texture: None,
        }
    }
}
