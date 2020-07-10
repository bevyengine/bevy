use crate::ColorMaterial;
use bevy_asset::{Assets, Handle};
use bevy_core::bytes::Byteable;
use bevy_render::{
    render_resource::{RenderResource, RenderResources},
    texture::Texture,
};
use glam::Vec2;
use bevy_ecs::{Res, Query};

#[repr(C)]
#[derive(Default, RenderResources, RenderResource)]
#[render_resources(from_self)]
pub struct Sprite {
    pub size: Vec2,
}

// SAFE: sprite is repr(C) and only consists of byteables
unsafe impl Byteable for Sprite {}

pub fn sprite_system(
    materials: Res<Assets<ColorMaterial>>,
    textures: Res<Assets<Texture>>,
    mut query: Query<(&mut Sprite, &Handle<ColorMaterial>)>,
) {
    for (sprite, handle) in &mut query.iter() {
        let material = materials.get(&handle).unwrap();
        if let Some(texture_handle) = material.texture {
            if let Some(texture) = textures.get(&texture_handle) {
                sprite.size = texture.size;
            }
        }
    }
}
