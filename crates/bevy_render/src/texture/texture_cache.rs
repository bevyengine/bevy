use crate::{
    render_resource::{Texture, TextureView},
    renderer::RenderDevice,
};
use bevy_ecs::{prelude::ResMut, resource::Resource};
use bevy_platform::collections::{hash_map::RawEntryMut, Equivalent, HashMap};
use wgpu::TextureViewDescriptor;

/// The internal representation of a [`CachedTexture`] used to track whether it was recently used
/// and is currently taken.
struct CachedTextureMeta {
    texture: Texture,
    default_view: TextureView,
    taken: bool,
    frames_since_last_use: usize,
}

/// A cached GPU [`Texture`] with corresponding [`TextureView`].
///
/// This is useful for textures that are created repeatedly (each frame) in the rendering process
/// to reduce the amount of GPU memory allocations.
#[derive(Clone)]
pub struct CachedTexture {
    pub texture: Texture,
    pub default_view: TextureView,
}

/// This resource caches textures that are created repeatedly in the rendering process and
/// are only required for one frame.
#[derive(Resource, Default)]
pub struct TextureCache {
    textures: HashMap<TextureCacheKey, Vec<CachedTextureMeta>>,
}

#[derive(Hash, PartialEq, Eq)]
struct TextureCacheKey(
    wgpu_types::TextureDescriptor<
        Option<String>,
        smallvec::SmallVec<[wgpu_types::TextureFormat; 1]>,
    >,
);

impl Equivalent<TextureCacheKey> for wgpu::TextureDescriptor<'_> {
    fn equivalent(&self, key: &TextureCacheKey) -> bool {
        self == &key
            .0
            .map_label_and_view_formats(|l| l.as_deref(), AsRef::as_ref)
    }
}

impl TextureCache {
    /// Retrieves a texture that matches the `descriptor`. If no matching one is found a new
    /// [`CachedTexture`] is created.
    pub fn get(
        &mut self,
        render_device: &RenderDevice,
        descriptor: wgpu::TextureDescriptor<'_>,
    ) -> CachedTexture {
        match self.textures.raw_entry_mut().from_key(&descriptor) {
            RawEntryMut::Occupied(mut entry) => {
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

                let texture = render_device.create_texture(
                    &entry
                        .key()
                        .0
                        .map_label_and_view_formats(|l| l.as_deref(), AsRef::as_ref),
                );
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
            RawEntryMut::Vacant(entry) => {
                let texture = render_device.create_texture(&descriptor);
                let default_view = texture.create_view(&TextureViewDescriptor::default());
                entry.insert(
                    TextureCacheKey(descriptor.map_label_and_view_formats(
                        |l| l.map(ToString::to_string),
                        |v| smallvec::SmallVec::from(*v),
                    )),
                    vec![CachedTextureMeta {
                        texture: texture.clone(),
                        default_view: default_view.clone(),
                        taken: true,
                        frames_since_last_use: 0,
                    }],
                );
                CachedTexture {
                    texture,
                    default_view,
                }
            }
        }
    }

    /// Returns `true` if the texture cache contains no textures.
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Updates the cache and only retains recently used textures.
    pub fn update(&mut self) {
        self.textures.retain(|_, textures| {
            for texture in textures.iter_mut() {
                texture.frames_since_last_use += 1;
                texture.taken = false;
            }

            textures.retain(|texture| texture.frames_since_last_use < 3);
            !textures.is_empty()
        });
    }
}

/// Updates the [`TextureCache`] to only retains recently used textures.
pub fn update_texture_cache_system(mut texture_cache: ResMut<TextureCache>) {
    texture_cache.update();
}
