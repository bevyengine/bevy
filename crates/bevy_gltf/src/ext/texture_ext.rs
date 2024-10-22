use bevy_asset::{Handle, LoadContext};
use bevy_image::Image;

use crate::{data_uri::DataUri, GltfAssetLabel};

pub trait TextureExt {
    fn get_texture_from_asset_label(&self, load_context: &mut LoadContext) -> Handle<Image>;
}

impl TextureExt for gltf::Texture<'_> {
    fn get_texture_from_asset_label(&self, load_context: &mut LoadContext) -> Handle<Image> {
        match self.source().source() {
            gltf::image::Source::View { .. } => {
                load_context.get_label_handle(GltfAssetLabel::Texture(self.index()).to_string())
            }
            gltf::image::Source::Uri { uri, .. } => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                if let Ok(_data_uri) = DataUri::parse(uri) {
                    load_context.get_label_handle(GltfAssetLabel::Texture(self.index()).to_string())
                } else {
                    let parent = load_context.path().parent().unwrap();
                    let image_path = parent.join(uri);
                    load_context.load(image_path)
                }
            }
        }
    }
}
