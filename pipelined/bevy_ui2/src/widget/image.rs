use crate::{CalculatedSize, UiImage};
use bevy_asset::Assets;
use bevy_ecs::{
    component::Component,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_math::Size;
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_render2::texture::Image;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect_value(Component, Serialize, Deserialize)]
pub enum ImageMode {
    KeepAspect,
}

impl Default for ImageMode {
    fn default() -> Self {
        ImageMode::KeepAspect
    }
}

pub fn image_node_system(
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut CalculatedSize, &UiImage), With<ImageMode>>,
) {
    for (mut calculated_size, image) in query.iter_mut() {
        if let Some(texture) = textures.get(image.0.clone_weak()) {
            let size = Size {
                width: texture.texture_descriptor.size.width as f32,
                height: texture.texture_descriptor.size.height as f32,
            };
            // Update only if size has changed to avoid needless layout calculations
            if size != calculated_size.size {
                calculated_size.size = size;
            }
        }
    }
}
