use crate::ColorMaterial;
use bevy_asset::{Assets, Handle};
use bevy_core::bytes::Byteable;
use bevy_render::{
    render_resource::{RenderResource, RenderResources},
    texture::Texture,
};
use legion::prelude::*;
use glam::Vec2;

#[repr(C)]
#[derive(Default, RenderResources, RenderResource)]
#[render_resources(from_self)]
pub struct Sprite {
    pub size: Vec2,
}

// SAFE: sprite is repr(C) and only consists of byteables
unsafe impl Byteable for Sprite {}

// TODO: port to system fn
pub fn sprite_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("sprite_system")
        .read_resource::<Assets<ColorMaterial>>()
        .read_resource::<Assets<Texture>>()
        .with_query(<(Write<Sprite>, Read<Handle<ColorMaterial>>)>::query())
        .build(|_, world, (materials, textures), query| {
            for (mut sprite, handle) in query.iter_mut(world) {
                let material = materials.get(&handle).unwrap();
                if let Some(texture_handle) = material.texture {
                    if let Some(texture) = textures.get(&texture_handle) {
                        sprite.size = texture.size;
                    }
                }
            }
        })
}
