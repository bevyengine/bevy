use crate::CalculatedSize;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Query, Res};
use bevy_math::Size;
use bevy_render::texture::Texture;
use bevy_sprite::ColorMaterial;

#[derive(Debug, Clone)]
pub enum Image {
    KeepAspect,
}

impl Default for Image {
    fn default() -> Self {
        Image::KeepAspect
    }
}

pub fn image_node_system(
    materials: Res<Assets<ColorMaterial>>,
    textures: Res<Assets<Texture>>,
    mut query: Query<(&Image, &mut CalculatedSize, &Handle<ColorMaterial>)>,
) {
    for (_image, mut calculated_size, material_handle) in &mut query.iter() {
        if let Some(texture) = materials
            .get(material_handle)
            .and_then(|material| material.texture)
            .and_then(|texture_handle| textures.get(&texture_handle))
        {
            calculated_size.size = Size {
                width: texture.size.x(),
                height: texture.size.y(),
            };
        }
    }
}
