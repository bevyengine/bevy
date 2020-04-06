use crate::{texture::Texture, Color};
use bevy_asset::Handle;
use bevy_derive::Uniforms;

use bevy_asset;
use bevy_core;

#[derive(Uniforms)]
#[module(meta = false, bevy_render = "crate")]
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
