use crate::{texture::Texture, Color};
use bevy_asset::Handle;
use bevy_derive::Uniforms;

use bevy_core;
use bevy_asset;

#[derive(Uniforms)]
#[uniform(bevy_render_path = "crate", bevy_core_path = "bevy_core", bevy_asset_path = "bevy_asset")]
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
