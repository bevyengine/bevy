use crate::{
    asset::Handle,
    render::{texture::Texture, Color},
};

use bevy_derive::Uniforms;

#[derive(Uniforms)]
#[uniform(bevy_path = "crate")]
pub struct StandardMaterial {
    #[uniform(instance)]
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
