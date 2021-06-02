use crate::{
    render_resource::{TextureId, TextureViewId},
    renderer::RenderResources,
    texture::{TextureDescriptor, TextureViewDescriptor},
};
use bevy_ecs::prelude::{Res, ResMut};
use bevy_utils::HashMap;

struct CachedTextureMeta {
    texture: TextureId,
    default_view: TextureViewId,
    taken: bool,
    frames_since_last_use: usize,
}

pub struct CachedTexture {
    pub texture: TextureId,
    pub default_view: TextureViewId,
}

#[derive(Default)]
pub struct TextureCache {
    textures: HashMap<TextureDescriptor, Vec<CachedTextureMeta>>,
}

impl TextureCache {
    pub fn get(
        &mut self,
        render_resources: &RenderResources,
        descriptor: TextureDescriptor,
    ) -> CachedTexture {
        match self.textures.entry(descriptor) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                for texture in entry.get_mut().iter_mut() {
                    if !texture.taken {
                        texture.frames_since_last_use = 0;
                        texture.taken = true;
                        return CachedTexture {
                            texture: texture.texture,
                            default_view: texture.default_view,
                        };
                    }
                }

                let texture_id = render_resources.create_texture(entry.key().clone());
                let view_id = render_resources
                    .create_texture_view(texture_id, TextureViewDescriptor::default());
                entry.get_mut().push(CachedTextureMeta {
                    texture: texture_id,
                    default_view: view_id,
                    frames_since_last_use: 0,
                    taken: true,
                });
                CachedTexture {
                    texture: texture_id,
                    default_view: view_id,
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let texture_id = render_resources.create_texture(entry.key().clone());
                let view_id = render_resources
                    .create_texture_view(texture_id, TextureViewDescriptor::default());
                entry.insert(vec![CachedTextureMeta {
                    texture: texture_id,
                    default_view: view_id,
                    taken: true,
                    frames_since_last_use: 0,
                }]);
                CachedTexture {
                    texture: texture_id,
                    default_view: view_id,
                }
            }
        }
    }

    pub fn update(&mut self, render_resources: &RenderResources) {
        for textures in self.textures.values_mut() {
            for texture in textures.iter_mut() {
                texture.frames_since_last_use += 1;
                texture.taken = false;
            }

            textures.retain(|texture| {
                let should_keep = texture.frames_since_last_use < 3;
                if !should_keep {
                    render_resources.remove_texture_view(texture.default_view);
                    render_resources.remove_texture(texture.texture);
                }
                should_keep
            });
        }
    }
}

pub fn update_texture_cache_system(
    mut texture_cache: ResMut<TextureCache>,
    render_resources: Res<RenderResources>,
) {
    texture_cache.update(&render_resources);
}
