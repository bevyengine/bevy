use bevy_asset::{self, Handle};
use bevy_derive::Uniforms;
use bevy_render::{texture::Texture, Color};

#[derive(Uniforms)]
pub struct StandardMaterial {
    pub albedo: Color,
    #[uniform(shader_def)]
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
