use bevy_asset::{Handle, LoadContext};
use bevy_image::{Image, ImageAddressMode, ImageFilterMode, ImageSamplerDescriptor};
use bevy_math::Affine2;

use gltf::texture::{MagFilter, MinFilter, Texture, TextureTransform, WrappingMode};

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures",
    feature = "pbr_specular_textures"
))]
use gltf::{json::texture::Info, Document};

use crate::loader::{LabelOrAssetPath, LoadedTexture};

// XXX TODO: Documentation.
pub(crate) fn texture_handle(
    texture: &Texture<'_>,
    load_context: &mut LoadContext,
    loaded_textures: &[LoadedTexture],
) -> Handle<Image> {
    let loaded_texture = &loaded_textures[texture.index()]; // XXX TODO: Guard against invalid indices?

    let handle = match &loaded_texture.path {
        LabelOrAssetPath::Label(label) => load_context.get_label_handle(label.to_string()),
        LabelOrAssetPath::AssetPath(path) => load_context.load(path),
    };

    // XXX TODO: Document and decide on error handling.
    assert_eq!(handle, loaded_texture.handle);

    handle
}

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

#[cfg(any(
    feature = "pbr_anisotropy_texture",
    feature = "pbr_multi_layer_material_textures",
    feature = "pbr_specular_textures"
))]
/// Given a [`Info`], returns the handle of the texture that this
/// refers to.
///
/// This is a low-level function only used when the [`gltf`] crate has no support
/// for an extension, forcing us to parse its texture references manually.
pub(crate) fn texture_handle_from_info(
    info: &Info,
    document: &Document,
    load_context: &mut LoadContext,
    loaded_textures: &[LoadedTexture],
) -> Handle<Image> {
    let texture = document
        .textures()
        .nth(info.index.value())
        .expect("Texture info references a nonexistent texture");
    texture_handle(&texture, load_context, loaded_textures)
}
