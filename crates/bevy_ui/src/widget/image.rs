use crate::{CalculatedSize, Size, UiImage, Val};
use bevy_asset::Assets;
use bevy_ecs::{
    component::Component,
    query::{With, Without},
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::texture::Image;
use bevy_text::Text;
use serde::{Deserialize, Serialize};

/// Describes how to resize the Image node
#[derive(Component, Debug, Default, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub enum ImageMode {
    /// Keep the aspect ratio of the image
    #[default]
    KeepAspect,
}

/// Updates calculated size of the node based on the image provided
pub fn image_node_system(
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut CalculatedSize, &UiImage), (With<ImageMode>, Without<Text>)>,
) {
    for (mut calculated_size, image) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let size = Size {
                width: Val::Px(texture.texture_descriptor.size.width as f32),
                height: Val::Px(texture.texture_descriptor.size.height as f32),
            };
            // Update only if size has changed to avoid needless layout calculations
            if size != calculated_size.size {
                calculated_size.size = size;
            }
        }
    }
}
