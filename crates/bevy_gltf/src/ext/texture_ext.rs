use std::path::Path;

use gltf::texture::{MagFilter, MinFilter};

use bevy_asset::{Handle, LoadContext, RenderAssetUsages};
use bevy_image::{
    CompressedImageFormats, Image, ImageFilterMode, ImageSampler, ImageSamplerDescriptor, ImageType,
};
use bevy_utils::HashSet;

use crate::{data_uri::DataUri, image_or_path::ImageOrPath, GltfAssetLabel, GltfError};

use super::WrappingModeExt;

pub trait TextureExt {
    /// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
    async fn load_texture<'a, 'b>(
        &self,
        buffer_data: &[Vec<u8>],
        linear_textures: &HashSet<usize>,
        parent_path: &'b Path,
        supported_compressed_formats: CompressedImageFormats,
        render_asset_usages: RenderAssetUsages,
    ) -> Result<ImageOrPath, GltfError>;

    /// Extracts the texture sampler data from the glTF texture.
    fn texture_sampler(&self) -> ImageSamplerDescriptor;

    fn get_texture_from_asset_label(&self, load_context: &mut LoadContext) -> Handle<Image>;
}

impl TextureExt for gltf::Texture<'_> {
    /// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
    async fn load_texture<'a, 'b>(
        &self,
        buffer_data: &[Vec<u8>],
        linear_textures: &HashSet<usize>,
        parent_path: &'b Path,
        supported_compressed_formats: CompressedImageFormats,
        render_asset_usages: RenderAssetUsages,
    ) -> Result<ImageOrPath, GltfError> {
        let is_srgb = !linear_textures.contains(&self.index());
        let sampler_descriptor = self.texture_sampler();
        #[cfg(all(debug_assertions, feature = "dds"))]
        let name = self
            .name()
            .map_or("Unknown GLTF Texture".to_string(), ToString::to_string);
        match self.source().source() {
            gltf::image::Source::View { view, mime_type } => {
                let start = view.offset();
                let end = view.offset() + view.length();
                let buffer = &buffer_data[view.buffer().index()][start..end];
                let image = Image::from_buffer(
                    #[cfg(all(debug_assertions, feature = "dds"))]
                    name,
                    buffer,
                    ImageType::MimeType(mime_type),
                    supported_compressed_formats,
                    is_srgb,
                    ImageSampler::Descriptor(sampler_descriptor),
                    render_asset_usages,
                )?;
                Ok(ImageOrPath::Image {
                    image,
                    label: GltfAssetLabel::Texture(self.index()),
                })
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                if let Ok(data_uri) = DataUri::parse(uri) {
                    let bytes = data_uri.decode()?;
                    let image_type = ImageType::MimeType(data_uri.mime_type);
                    let image = Image::from_buffer(
                        #[cfg(all(debug_assertions, feature = "dds"))]
                        name,
                        &bytes,
                        mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                        supported_compressed_formats,
                        is_srgb,
                        ImageSampler::Descriptor(sampler_descriptor),
                        render_asset_usages,
                    )?;
                    Ok(ImageOrPath::Image {
                        image,
                        label: GltfAssetLabel::Texture(self.index()),
                    })
                } else {
                    let image_path = parent_path.join(uri);
                    Ok(ImageOrPath::Path {
                        path: image_path,
                        is_srgb,
                        sampler_descriptor,
                    })
                }
            }
        }
    }

    /// Extracts the texture sampler data from the glTF texture.
    fn texture_sampler(&self) -> ImageSamplerDescriptor {
        let gltf_sampler = self.sampler();

        ImageSamplerDescriptor {
            address_mode_u: gltf_sampler.wrap_s().texture_address_mode(),
            address_mode_v: gltf_sampler.wrap_t().texture_address_mode(),

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
