use crate::CalculatedSize;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    component::Component,
    query::With,
    system::{Query, Res},
};
use bevy_math::Size;
use bevy_render::texture::Texture;
use bevy_sprite::ColorMaterial;

#[derive(Component, Debug, Clone)]
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
    mut query: Query<(&mut CalculatedSize, &Handle<ColorMaterial>), With<Image>>,
) {
    for (mut calculated_size, material_handle) in query.iter_mut() {
        if let Some(texture) = materials
            .get(material_handle)
            .and_then(|material| material.texture.as_ref())
            .and_then(|texture_handle| textures.get(texture_handle))
        {
            let size = Size {
                width: texture.size.width as f32,
                height: texture.size.height as f32,
            };
            // Update only if size has changed to avoid needless layout calculations
            if size != calculated_size.size {
                calculated_size.size = size;
            }
        }
    }
}
