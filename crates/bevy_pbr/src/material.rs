use bevy_asset::{self, Handle};
use bevy_reflect::TypeUuid;
use bevy_render::{color::Color, renderer::RenderResources, shader::ShaderDefs, texture::Texture};

/// A material with "standard" properties used in PBR lighting
#[derive(Debug, RenderResources, ShaderDefs, TypeUuid)]
#[uuid = "dace545e-4bc6-4595-a79d-c224fc694975"]
pub struct StandardMaterial {
    pub base_color_factor: Color,
    #[shader_def]
    pub base_color_texture: Option<Handle<Texture>>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    #[render_resources(ignore)]
    #[shader_def]
    pub unlit: bool,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            base_color_factor: Color::rgb(1.0, 1.0, 1.0),
            base_color_texture: None,
            roughness_factor: 0.01,
            metallic_factor: 0.08,
            unlit: false,
        }
    }
}

impl From<Color> for StandardMaterial {
    fn from(color: Color) -> Self {
        StandardMaterial {
            base_color_factor: color,
            ..Default::default()
        }
    }
}

impl From<Handle<Texture>> for StandardMaterial {
    fn from(texture: Handle<Texture>) -> Self {
        StandardMaterial {
            base_color_texture: Some(texture),
            ..Default::default()
        }
    }
}
