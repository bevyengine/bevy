use bevy_asset::{Handle, LoadContext};
use bevy_image::Image;

use crate::ext::TextureExt;

pub trait JsonTextureInfoExt {
    fn texture_handle_from_info(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
    ) -> Handle<Image>;
}

impl JsonTextureInfoExt for gltf::json::texture::Info {
    fn texture_handle_from_info(
        &self,
        load_context: &mut LoadContext,
        document: &gltf::Document,
    ) -> Handle<Image> {
        let texture = document
            .textures()
            .nth(self.index.value())
            .expect("Texture info references a nonexistent texture");
        texture.get_texture_from_asset_label(load_context)
    }
}
