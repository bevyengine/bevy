use crate::{math::Vec4, render::ColorSource};

use crate as bevy; // for macro imports
use bevy_derive::Uniforms;

#[derive(Uniforms)]
pub struct StandardMaterial {
    pub albedo: ColorSource,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            albedo: ColorSource::Color(Vec4::new(0.3, 0.3, 0.3, 1.0)),
        }
    }
}
