pub mod transform;
pub mod wrapping_mode;

use bevy_asset::{Handle, LoadContext};
use bevy_image::{Image, ImageFilterMode, ImageSamplerDescriptor};
use gltf::{
    image::Source,
    texture::{MagFilter, MinFilter},
};

use crate::{loader::data_uri::DataUri, GltfAssetLabel};

use wrapping_mode::WrappingModeExt;

pub trait TextureExt {
    fn handle(&self, load_context: &mut LoadContext) -> Handle<Image>;

    fn texture_sampler(&self) -> ImageSamplerDescriptor;
}

impl TextureExt for gltf::Texture<'_> {
    fn handle(&self, load_context: &mut LoadContext) -> Handle<Image> {
        match self.source().source() {
            Source::View { .. } => {
                load_context.get_label_handle(GltfAssetLabel::from(self).to_string())
            }
            Source::Uri { uri, .. } => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                if let Ok(_data_uri) = DataUri::parse(uri) {
                    load_context.get_label_handle(GltfAssetLabel::from(self).to_string())
                } else {
                    let parent = load_context.path().parent().unwrap();
                    let image_path = parent.join(uri);
                    load_context.load(image_path)
                }
            }
        }
    }

    /// Extracts the texture sampler data from the glTF texture.
    fn texture_sampler(&self) -> ImageSamplerDescriptor {
        let gltf_sampler = self.sampler();

        ImageSamplerDescriptor {
            address_mode_u: gltf_sampler.wrap_s().address_mode(),
            address_mode_v: gltf_sampler.wrap_t().address_mode(),

            mag_filter: gltf_sampler
                .mag_filter()
                .map(|mf| match mf {
                    MagFilter::Nearest => ImageFilterMode::Nearest,
                    MagFilter::Linear => ImageFilterMode::Linear,
                })
                .unwrap_or(ImageSamplerDescriptor::default().mag_filter),

            min_filter: gltf_sampler
                .min_filter()
                .map(|mf| match mf {
                    MinFilter::Nearest
                    | MinFilter::NearestMipmapNearest
                    | MinFilter::NearestMipmapLinear => ImageFilterMode::Nearest,
                    MinFilter::Linear
                    | MinFilter::LinearMipmapNearest
                    | MinFilter::LinearMipmapLinear => ImageFilterMode::Linear,
                })
                .unwrap_or(ImageSamplerDescriptor::default().min_filter),

            mipmap_filter: gltf_sampler
                .min_filter()
                .map(|mf| match mf {
                    MinFilter::Nearest
                    | MinFilter::Linear
                    | MinFilter::NearestMipmapNearest
                    | MinFilter::LinearMipmapNearest => ImageFilterMode::Nearest,
                    MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                        ImageFilterMode::Linear
                    }
                })
                .unwrap_or(ImageSamplerDescriptor::default().mipmap_filter),

            ..Default::default()
        }
    }
}
