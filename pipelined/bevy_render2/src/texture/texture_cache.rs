use crate::{
    render_resource::{Texture, TextureView},
    renderer::RenderDevice,
};
use bevy_ecs::prelude::ResMut;
use bevy_utils::HashMap;
use wgpu::{TextureDescriptor, TextureViewDescriptor};

struct CachedTextureMeta {
    texture: Texture,
    default_view: TextureView,
    taken: bool,
    frames_since_last_use: usize,
}

pub struct CachedTexture {
    pub texture: Texture,
    pub default_view: TextureView,
}

#[derive(Default)]
pub struct TextureCache {
    textures: HashMap<wgpu::TextureDescriptor<'static>, Vec<CachedTextureMeta>>,
}

impl TextureCache {
    pub fn get(
        &mut self,
        render_device: &RenderDevice,
        descriptor: TextureDescriptor<'static>,
    ) -> CachedTexture {
        match self.textures.entry(descriptor) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                for texture in entry.get_mut().iter_mut() {
                    if !texture.taken {
                        texture.frames_since_last_use = 0;
                        texture.taken = true;
                        return CachedTexture {
                            texture: texture.texture.clone(),
                            default_view: texture.default_view.clone(),
                        };
                    }
                }

                let texture = render_device.create_texture(&entry.key().clone());
                let default_view = texture.create_view(&TextureViewDescriptor::default());
                entry.get_mut().push(CachedTextureMeta {
                    texture: texture.clone(),
                    default_view: default_view.clone(),
                    frames_since_last_use: 0,
                    taken: true,
                });
                CachedTexture {
                    texture,
                    default_view,
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let texture = render_device.create_texture(entry.key());
                let default_view = texture.create_view(&TextureViewDescriptor::default());
                entry.insert(vec![CachedTextureMeta {
                    texture: texture.clone(),
                    default_view: default_view.clone(),
                    taken: true,
                    frames_since_last_use: 0,
                }]);
                CachedTexture {
                    texture,
                    default_view,
                }
            }
        }
    }

    pub fn update(&mut self) {
        for textures in self.textures.values_mut() {
            for texture in textures.iter_mut() {
                texture.frames_since_last_use += 1;
                texture.taken = false;
            }

            textures.retain(|texture| texture.frames_since_last_use < 3);
        }
    }
}

pub fn update_texture_cache_system(mut texture_cache: ResMut<TextureCache>) {
    texture_cache.update();
}
