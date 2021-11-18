use crate::CalculatedSize;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    query::With,
    system::{Query, Res},
};
use bevy_math::Size;

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
    textures: Res<Assets<bevy_render2::texture::Image>>,
    mut query: Query<
        (
            &mut CalculatedSize,
            &Option<Handle<bevy_render2::texture::Image>>,
        ),
        With<Image>,
    >,
) {
    for (mut calculated_size, texture_handle) in query.iter_mut() {
        if let Some(texture) = texture_handle
            .as_ref()
            .and_then(|handle| textures.get(handle))
        {
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
