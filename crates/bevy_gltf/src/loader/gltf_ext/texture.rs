use bevy_image::{ImageAddressMode, ImageFilterMode, ImageLoader, ImageSamplerDescriptor};
use bevy_math::Affine2;

use gltf::{
    image::Image,
    texture::{MagFilter, MinFilter, Texture, TextureTransform, WrappingMode},
    Document,
};
use serde_json::Value;

/// Extracts the texture sampler data from the glTF [`Texture`].
pub(crate) fn texture_sampler(
    texture: &Texture<'_>,
    default_sampler: &ImageSamplerDescriptor,
) -> ImageSamplerDescriptor {
    let gltf_sampler = texture.sampler();
    let mut sampler = default_sampler.clone();

    sampler.address_mode_u = address_mode(&gltf_sampler.wrap_s());
    sampler.address_mode_v = address_mode(&gltf_sampler.wrap_t());

    // Shouldn't parse filters when anisotropic filtering is on, because trilinear is then required by wgpu.
    // We also trust user to have provided a valid sampler.
    if sampler.anisotropy_clamp == 1 {
        if let Some(mag_filter) = gltf_sampler.mag_filter().map(|mf| match mf {
            MagFilter::Nearest => ImageFilterMode::Nearest,
            MagFilter::Linear => ImageFilterMode::Linear,
        }) {
            sampler.mag_filter = mag_filter;
        }
        if let Some(min_filter) = gltf_sampler.min_filter().map(|mf| match mf {
            MinFilter::Nearest
            | MinFilter::NearestMipmapNearest
            | MinFilter::NearestMipmapLinear => ImageFilterMode::Nearest,
            MinFilter::Linear | MinFilter::LinearMipmapNearest | MinFilter::LinearMipmapLinear => {
                ImageFilterMode::Linear
            }
        }) {
            sampler.min_filter = min_filter;
        }
        if let Some(mipmap_filter) = gltf_sampler.min_filter().map(|mf| match mf {
            MinFilter::Nearest
            | MinFilter::Linear
            | MinFilter::NearestMipmapNearest
            | MinFilter::LinearMipmapNearest => ImageFilterMode::Nearest,
            MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                ImageFilterMode::Linear
            }
        }) {
            sampler.mipmap_filter = mipmap_filter;
        }
    }
    sampler
}

pub(crate) fn texture_source<'a>(
    texture: &Texture<'a>,
    document: &'a Document,
) -> Result<Option<Image<'a>>, String> {
    // These are pure opinionated value judgment. Higher is better lower is worse.
    // The base case is always last but guaranteed.
    // The extensions we rank.
    const BASE_TEXTURE_SOURCE_WEIGHT: u8 = 0;
    const KHR_TEXTURE_BASISU_WEIGHT: u8 = 200;

    // Base case here. No matter what we have something. This could be a ktx2 technically
    // but valid ktx2.0 files will only have pngs and jpegs here.
    let mut images = Vec::new();
    if let Some(image) = texture.source() {
        images.push((BASE_TEXTURE_SOURCE_WEIGHT, image));
    }

    // This block is where we check if we support ktx2 and if we do we add it with the weight.
    if ImageLoader::SUPPORTED_FILE_EXTENSIONS.contains(&"ktx2")
        && let Some(extension) = texture.extension_value("KHR_texture_basisu")
    {
        let source = extension
            .get("source")
            .and_then(Value::as_u64)
            .and_then(|source| usize::try_from(source).ok())
            .ok_or_else(|| extension.to_string())?;

        let ktx2_image = document
            .images()
            .nth(source)
            .ok_or_else(|| source.to_string())?;

        images.push((KHR_TEXTURE_BASISU_WEIGHT, ktx2_image));
    }

    // We grab the highest weight and return it.
    let image = images
        .into_iter()
        .max_by_key(|(weight, _)| *weight)
        .map(|(_, image)| image);

    Ok(image)
}

pub(crate) fn address_mode(wrapping_mode: &WrappingMode) -> ImageAddressMode {
    match wrapping_mode {
        WrappingMode::ClampToEdge => ImageAddressMode::ClampToEdge,
        WrappingMode::Repeat => ImageAddressMode::Repeat,
        WrappingMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
    }
}

pub(crate) fn texture_transform_to_affine2(texture_transform: TextureTransform) -> Affine2 {
    Affine2::from_scale_angle_translation(
        texture_transform.scale().into(),
        -texture_transform.rotation(),
        texture_transform.offset().into(),
    )
}
