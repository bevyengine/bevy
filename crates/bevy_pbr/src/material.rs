use bevy_asset::{self, Handle};
use bevy_render::{color::Color, renderer::RenderResources, shader::ShaderDefs, texture::Texture};

/// A material with "standard" properties used in PBR lighting
#[derive(Debug, RenderResources, ShaderDefs)]
#[allow(clippy::manual_non_exhaustive)]
pub struct StandardMaterial {
    pub albedo: Color,
    #[shader_def]
    pub albedo_texture: Option<Handle<Texture>>,
    #[render_resources(ignore)]
    #[shader_def]
    pub shaded: bool,

    // this is a manual implementation of the non exhaustive pattern,
    // especially made to allow ..Default::default()
    #[render_resources(ignore)]
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            albedo: Color::rgb(1.0, 1.0, 1.0),
            albedo_texture: None,
            shaded: true,
            __non_exhaustive: (),
        }
    }
}

impl From<Color> for StandardMaterial {
    fn from(color: Color) -> Self {
        StandardMaterial {
            albedo: color,
            ..Default::default()
        }
    }
}

impl From<Handle<Texture>> for StandardMaterial {
    fn from(texture: Handle<Texture>) -> Self {
        StandardMaterial {
            albedo_texture: Some(texture),
            ..Default::default()
        }
    }
}
