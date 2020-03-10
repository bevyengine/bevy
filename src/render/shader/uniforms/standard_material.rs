use crate::render::{Color, ColorSource};

use crate as bevy; // for macro imports
use bevy_derive::Uniforms;

#[derive(Uniforms)]
pub struct StandardMaterial {
    #[uniform(shader_def)]
    pub albedo: ColorSource,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            albedo: Color::rgb(0.3, 0.3, 0.3).into(),
        }
    }
}
