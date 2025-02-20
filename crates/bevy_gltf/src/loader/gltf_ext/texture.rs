use bevy_asset::{Handle, LoadContext};
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor};
use bevy_math::Affine2;

use gltf::{
    image::Source,
    texture::{MagFilter, MinFilter, Texture, TextureTransform, WrappingMode},
};

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures",
    feature = "pbr_specular_textures"
))]
use gltf::{json::texture::Info, Document};

use crate::{loader::data_uri::DataUri, GltfAssetLabel};

pub trait TextureExt {
    fn handle(&self, load_context: &mut LoadContext) -> Handle<Image>;

    fn texture_sampler(&self) -> ImageSamplerDescriptor;
}

pub trait WrappingModeExt {
    fn address_mode(&self) -> ImageAddressMode;
}

pub trait TextureTransformExt {
    fn to_affine2(self) -> Affine2;
}

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures",
    feature = "pbr_specular_textures"
))]
pub trait InfoExt {
    fn texture_handle(&self, document: &Document, load_context: &mut LoadContext) -> Handle<Image>;
}

impl TextureExt for Texture<'_> {
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

impl WrappingModeExt for WrappingMode {
    /// Maps the texture address mode from glTF to wgpu.
    fn address_mode(&self) -> ImageAddressMode {
        match self {
            WrappingMode::ClampToEdge => ImageAddressMode::ClampToEdge,
            WrappingMode::Repeat => ImageAddressMode::Repeat,
            WrappingMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
        }
    }
}

impl TextureTransformExt for TextureTransform<'_> {
    fn to_affine2(self) -> Affine2 {
        Affine2::from_scale_angle_translation(
            self.scale().into(),
            -self.rotation(),
            self.offset().into(),
        )
    }
}

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures",
    feature = "pbr_specular_textures"
))]
impl InfoExt for Info {
    /// Given a [`Info`], returns the handle of the texture that this
    /// refers to.
    ///
    /// This is a low-level function only used when the [`gltf`] crate has no support
    /// for an extension, forcing us to parse its texture references manually.
    fn texture_handle(&self, document: &Document, load_context: &mut LoadContext) -> Handle<Image> {
        let texture = document
            .textures()
            .nth(self.index.value())
            .expect("Texture info references a nonexistent texture");
        texture.handle(load_context)
    }
}
