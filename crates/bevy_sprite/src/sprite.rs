use crate::{ColorMaterial, Quad};
use bevy_asset::{Assets, Handle};
use bevy_render::texture::Texture;
pub use legion::prelude::*;
pub struct Sprite {
    pub scale: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite { scale: 1.0 }
    }
}

pub fn sprite_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("sprite_system")
        .read_resource::<Assets<ColorMaterial>>()
        .read_resource::<Assets<Texture>>()
        .with_query(
            <(Read<Sprite>, Read<Handle<ColorMaterial>>, Write<Quad>)>::query().filter(
                changed::<Sprite>() | changed::<Quad>() | changed::<Handle<ColorMaterial>>(),
            ),
        )
        .build(|_, world, (materials, textures), query| {
            for (sprite, handle, mut rect) in query.iter_mut(world) {
                let material = materials.get(&handle).unwrap();
                if let Some(texture_handle) = material.texture {
                    if let Some(texture) = textures.get(&texture_handle) {
                        let aspect = texture.aspect();
                        *rect.size.x_mut() = texture.size.x() * sprite.scale;
                        *rect.size.y_mut() = rect.size.x() * aspect;
                    }
                }
            }
        })
}
