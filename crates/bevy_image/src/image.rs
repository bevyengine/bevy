#[cfg(feature = "basis-universal")]
use super::basis::*;
#[cfg(feature = "dds")]
use super::dds::*;
#[cfg(feature = "ktx2")]
use super::ktx2::*;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use bevy_asset::{Asset, RenderAssetUsages};
use bevy_color::{Color, ColorToComponents, Gray, LinearRgba, Srgba, Xyza};
use bevy_math::{AspectRatio, UVec2, UVec3, Vec2};
use core::hash::Hash;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu::{SamplerDescriptor, TextureViewDescriptor};
use wgpu_types::{
    AddressMode, CompareFunction, Extent3d, Features, FilterMode, SamplerBorderColor,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

pub trait BevyDefault {
    fn bevy_default() -> Self;
}

impl BevyDefault for TextureFormat {
    fn bevy_default() -> Self {
        TextureFormat::Rgba8UnormSrgb
    }
}

pub const TEXTURE_ASSET_INDEX: u64 = 0;
pub const SAMPLER_ASSET_INDEX: u64 = 1;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum ImageFormat {
    #[cfg(feature = "basis-universal")]
    Basis,
    #[cfg(feature = "bmp")]
    Bmp,
    #[cfg(feature = "dds")]
    Dds,
    #[cfg(feature = "ff")]
    Farbfeld,
    #[cfg(feature = "gif")]
    Gif,
    #[cfg(feature = "exr")]
    OpenExr,
    #[cfg(feature = "hdr")]
    Hdr,
    #[cfg(feature = "ico")]
    Ico,
    #[cfg(feature = "jpeg")]
    Jpeg,
    #[cfg(feature = "ktx2")]
    Ktx2,
    #[cfg(feature = "png")]
    Png,
    #[cfg(feature = "pnm")]
    Pnm,
    #[cfg(feature = "qoi")]
    Qoi,
    #[cfg(feature = "tga")]
    Tga,
    #[cfg(feature = "tiff")]
    Tiff,
    #[cfg(feature = "webp")]
    WebP,
}

macro_rules! feature_gate {
    ($feature: tt, $value: ident) => {{
        #[cfg(not(feature = $feature))]
        {
            tracing::warn!("feature \"{}\" is not enabled", $feature);
            return None;
        }
        #[cfg(feature = $feature)]
        ImageFormat::$value
    }};
}

impl ImageFormat {
    /// Gets the file extensions for a given format.
    pub const fn to_file_extensions(&self) -> &'static [&'static str] {
        match self {
            #[cfg(feature = "basis-universal")]
            ImageFormat::Basis => &["basis"],
            #[cfg(feature = "bmp")]
            ImageFormat::Bmp => &["bmp"],
            #[cfg(feature = "dds")]
            ImageFormat::Dds => &["dds"],
            #[cfg(feature = "ff")]
            ImageFormat::Farbfeld => &["ff", "farbfeld"],
            #[cfg(feature = "gif")]
            ImageFormat::Gif => &["gif"],
            #[cfg(feature = "exr")]
            ImageFormat::OpenExr => &["exr"],
            #[cfg(feature = "hdr")]
            ImageFormat::Hdr => &["hdr"],
            #[cfg(feature = "ico")]
            ImageFormat::Ico => &["ico"],
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => &["jpg", "jpeg"],
            #[cfg(feature = "ktx2")]
            ImageFormat::Ktx2 => &["ktx2"],
            #[cfg(feature = "pnm")]
            ImageFormat::Pnm => &["pam", "pbm", "pgm", "ppm"],
            #[cfg(feature = "png")]
            ImageFormat::Png => &["png"],
            #[cfg(feature = "qoi")]
            ImageFormat::Qoi => &["qoi"],
            #[cfg(feature = "tga")]
            ImageFormat::Tga => &["tga"],
            #[cfg(feature = "tiff")]
            ImageFormat::Tiff => &["tif", "tiff"],
            #[cfg(feature = "webp")]
            ImageFormat::WebP => &["webp"],
            // FIXME: https://github.com/rust-lang/rust/issues/129031
            #[expect(
                clippy::allow_attributes,
                reason = "`unreachable_patterns` may not always lint"
            )]
            #[allow(
                unreachable_patterns,
                reason = "The wildcard pattern will be unreachable if all formats are enabled; otherwise, it will be reachable"
            )]
            _ => &[],
        }
    }

    /// Gets the MIME types for a given format.
    ///
    /// If a format doesn't have any dedicated MIME types, this list will be empty.
    pub const fn to_mime_types(&self) -> &'static [&'static str] {
        match self {
            #[cfg(feature = "basis-universal")]
            ImageFormat::Basis => &["image/basis", "image/x-basis"],
            #[cfg(feature = "bmp")]
            ImageFormat::Bmp => &["image/bmp", "image/x-bmp"],
            #[cfg(feature = "dds")]
            ImageFormat::Dds => &["image/vnd-ms.dds"],
            #[cfg(feature = "hdr")]
            ImageFormat::Hdr => &["image/vnd.radiance"],
            #[cfg(feature = "gif")]
            ImageFormat::Gif => &["image/gif"],
            #[cfg(feature = "ff")]
            ImageFormat::Farbfeld => &[],
            #[cfg(feature = "ico")]
            ImageFormat::Ico => &["image/x-icon"],
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => &["image/jpeg"],
            #[cfg(feature = "ktx2")]
            ImageFormat::Ktx2 => &["image/ktx2"],
            #[cfg(feature = "png")]
            ImageFormat::Png => &["image/png"],
            #[cfg(feature = "qoi")]
            ImageFormat::Qoi => &["image/qoi", "image/x-qoi"],
            #[cfg(feature = "exr")]
            ImageFormat::OpenExr => &["image/x-exr"],
            #[cfg(feature = "pnm")]
            ImageFormat::Pnm => &[
                "image/x-portable-bitmap",
                "image/x-portable-graymap",
                "image/x-portable-pixmap",
                "image/x-portable-anymap",
            ],
            #[cfg(feature = "tga")]
            ImageFormat::Tga => &["image/x-targa", "image/x-tga"],
            #[cfg(feature = "tiff")]
            ImageFormat::Tiff => &["image/tiff"],
            #[cfg(feature = "webp")]
            ImageFormat::WebP => &["image/webp"],
            // FIXME: https://github.com/rust-lang/rust/issues/129031
            #[expect(
                clippy::allow_attributes,
                reason = "`unreachable_patterns` may not always lint"
            )]
            #[allow(
                unreachable_patterns,
                reason = "The wildcard pattern will be unreachable if all formats are enabled; otherwise, it will be reachable"
            )]
            _ => &[],
        }
    }

    pub fn from_mime_type(mime_type: &str) -> Option<Self> {
        #[expect(
            clippy::allow_attributes,
            reason = "`unreachable_code` may not always lint"
        )]
        #[allow(
            unreachable_code,
            reason = "If all features listed below are disabled, then all arms will have a `return None`, keeping the surrounding `Some()` from being constructed."
        )]
        Some(match mime_type.to_ascii_lowercase().as_str() {
            // note: farbfeld does not have a MIME type
            "image/basis" | "image/x-basis" => feature_gate!("basis-universal", Basis),
            "image/bmp" | "image/x-bmp" => feature_gate!("bmp", Bmp),
            "image/vnd-ms.dds" => feature_gate!("dds", Dds),
            "image/vnd.radiance" => feature_gate!("hdr", Hdr),
            "image/gif" => feature_gate!("gif", Gif),
            "image/x-icon" => feature_gate!("ico", Ico),
            "image/jpeg" => feature_gate!("jpeg", Jpeg),
            "image/ktx2" => feature_gate!("ktx2", Ktx2),
            "image/png" => feature_gate!("png", Png),
            "image/qoi" | "image/x-qoi" => feature_gate!("qoi", Qoi),
            "image/x-exr" => feature_gate!("exr", OpenExr),
            "image/x-portable-bitmap"
            | "image/x-portable-graymap"
            | "image/x-portable-pixmap"
            | "image/x-portable-anymap" => feature_gate!("pnm", Pnm),
            "image/x-targa" | "image/x-tga" => feature_gate!("tga", Tga),
            "image/tiff" => feature_gate!("tiff", Tiff),
            "image/webp" => feature_gate!("webp", WebP),
            _ => return None,
        })
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        #[expect(
            clippy::allow_attributes,
            reason = "`unreachable_code` may not always lint"
        )]
        #[allow(
            unreachable_code,
            reason = "If all features listed below are disabled, then all arms will have a `return None`, keeping the surrounding `Some()` from being constructed."
        )]
        Some(match extension.to_ascii_lowercase().as_str() {
            "basis" => feature_gate!("basis-universal", Basis),
            "bmp" => feature_gate!("bmp", Bmp),
            "dds" => feature_gate!("dds", Dds),
            "ff" | "farbfeld" => feature_gate!("ff", Farbfeld),
            "gif" => feature_gate!("gif", Gif),
            "exr" => feature_gate!("exr", OpenExr),
            "hdr" => feature_gate!("hdr", Hdr),
            "ico" => feature_gate!("ico", Ico),
            "jpg" | "jpeg" => feature_gate!("jpeg", Jpeg),
            "ktx2" => feature_gate!("ktx2", Ktx2),
            "pam" | "pbm" | "pgm" | "ppm" => feature_gate!("pnm", Pnm),
            "png" => feature_gate!("png", Png),
            "qoi" => feature_gate!("qoi", Qoi),
            "tga" => feature_gate!("tga", Tga),
            "tif" | "tiff" => feature_gate!("tiff", Tiff),
            "webp" => feature_gate!("webp", WebP),
            _ => return None,
        })
    }

    pub fn as_image_crate_format(&self) -> Option<image::ImageFormat> {
        #[expect(
            clippy::allow_attributes,
            reason = "`unreachable_code` may not always lint"
        )]
        #[allow(
            unreachable_code,
            reason = "If all features listed below are disabled, then all arms will have a `return None`, keeping the surrounding `Some()` from being constructed."
        )]
        Some(match self {
            #[cfg(feature = "bmp")]
            ImageFormat::Bmp => image::ImageFormat::Bmp,
            #[cfg(feature = "dds")]
            ImageFormat::Dds => image::ImageFormat::Dds,
            #[cfg(feature = "ff")]
            ImageFormat::Farbfeld => image::ImageFormat::Farbfeld,
            #[cfg(feature = "gif")]
            ImageFormat::Gif => image::ImageFormat::Gif,
            #[cfg(feature = "exr")]
            ImageFormat::OpenExr => image::ImageFormat::OpenExr,
            #[cfg(feature = "hdr")]
            ImageFormat::Hdr => image::ImageFormat::Hdr,
            #[cfg(feature = "ico")]
            ImageFormat::Ico => image::ImageFormat::Ico,
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => image::ImageFormat::Jpeg,
            #[cfg(feature = "png")]
            ImageFormat::Png => image::ImageFormat::Png,
            #[cfg(feature = "pnm")]
            ImageFormat::Pnm => image::ImageFormat::Pnm,
            #[cfg(feature = "qoi")]
            ImageFormat::Qoi => image::ImageFormat::Qoi,
            #[cfg(feature = "tga")]
            ImageFormat::Tga => image::ImageFormat::Tga,
            #[cfg(feature = "tiff")]
            ImageFormat::Tiff => image::ImageFormat::Tiff,
            #[cfg(feature = "webp")]
            ImageFormat::WebP => image::ImageFormat::WebP,
            #[cfg(feature = "basis-universal")]
            ImageFormat::Basis => return None,
            #[cfg(feature = "ktx2")]
            ImageFormat::Ktx2 => return None,
            // FIXME: https://github.com/rust-lang/rust/issues/129031
            #[expect(
                clippy::allow_attributes,
                reason = "`unreachable_patterns` may not always lint"
            )]
            #[allow(
                unreachable_patterns,
                reason = "The wildcard pattern will be unreachable if all formats are enabled; otherwise, it will be reachable"
            )]
            _ => return None,
        })
    }

    pub fn from_image_crate_format(format: image::ImageFormat) -> Option<ImageFormat> {
        #[expect(
            clippy::allow_attributes,
            reason = "`unreachable_code` may not always lint"
        )]
        #[allow(
            unreachable_code,
            reason = "If all features listed below are disabled, then all arms will have a `return None`, keeping the surrounding `Some()` from being constructed."
        )]
        Some(match format {
            image::ImageFormat::Bmp => feature_gate!("bmp", Bmp),
            image::ImageFormat::Dds => feature_gate!("dds", Dds),
            image::ImageFormat::Farbfeld => feature_gate!("ff", Farbfeld),
            image::ImageFormat::Gif => feature_gate!("gif", Gif),
            image::ImageFormat::OpenExr => feature_gate!("exr", OpenExr),
            image::ImageFormat::Hdr => feature_gate!("hdr", Hdr),
            image::ImageFormat::Ico => feature_gate!("ico", Ico),
            image::ImageFormat::Jpeg => feature_gate!("jpeg", Jpeg),
            image::ImageFormat::Png => feature_gate!("png", Png),
            image::ImageFormat::Pnm => feature_gate!("pnm", Pnm),
            image::ImageFormat::Qoi => feature_gate!("qoi", Qoi),
            image::ImageFormat::Tga => feature_gate!("tga", Tga),
            image::ImageFormat::Tiff => feature_gate!("tiff", Tiff),
            image::ImageFormat::WebP => feature_gate!("webp", WebP),
            _ => return None,
        })
    }
}

#[derive(Asset, Debug, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(opaque, Default, Debug)
)]
pub struct Image {
    pub data: Vec<u8>,
    // TODO: this nesting makes accessing Image metadata verbose. Either flatten out descriptor or add accessors
    pub texture_descriptor: TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>,
    /// The [`ImageSampler`] to use during rendering.
    pub sampler: ImageSampler,
    pub texture_view_descriptor: Option<TextureViewDescriptor<'static>>,
    pub asset_usage: RenderAssetUsages,
}

/// Used in [`Image`], this determines what image sampler to use when rendering. The default setting,
/// [`ImageSampler::Default`], will read the sampler from the `ImagePlugin` at setup.
/// Setting this to [`ImageSampler::Descriptor`] will override the global default descriptor for this [`Image`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum ImageSampler {
    /// Default image sampler, derived from the `ImagePlugin` setup.
    #[default]
    Default,
    /// Custom sampler for this image which will override global default.
    Descriptor(ImageSamplerDescriptor),
}

impl ImageSampler {
    /// Returns an image sampler with [`ImageFilterMode::Linear`] min and mag filters
    #[inline]
    pub fn linear() -> ImageSampler {
        ImageSampler::Descriptor(ImageSamplerDescriptor::linear())
    }

    /// Returns an image sampler with [`ImageFilterMode::Nearest`] min and mag filters
    #[inline]
    pub fn nearest() -> ImageSampler {
        ImageSampler::Descriptor(ImageSamplerDescriptor::nearest())
    }

    /// Initialize the descriptor if it is not already initialized.
    ///
    /// Descriptor is typically initialized by Bevy when the image is loaded,
    /// so this is convenient shortcut for updating the descriptor.
    pub fn get_or_init_descriptor(&mut self) -> &mut ImageSamplerDescriptor {
        match self {
            ImageSampler::Default => {
                *self = ImageSampler::Descriptor(ImageSamplerDescriptor::default());
                match self {
                    ImageSampler::Descriptor(descriptor) => descriptor,
                    _ => unreachable!(),
                }
            }
            ImageSampler::Descriptor(descriptor) => descriptor,
        }
    }
}

/// How edges should be handled in texture addressing.
///
/// See [`ImageSamplerDescriptor`] for information how to configure this.
///
/// This type mirrors [`AddressMode`].
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum ImageAddressMode {
    /// Clamp the value to the edge of the texture.
    ///
    /// -0.25 -> 0.0
    /// 1.25  -> 1.0
    #[default]
    ClampToEdge,
    /// Repeat the texture in a tiling fashion.
    ///
    /// -0.25 -> 0.75
    /// 1.25 -> 0.25
    Repeat,
    /// Repeat the texture, mirroring it every repeat.
    ///
    /// -0.25 -> 0.25
    /// 1.25 -> 0.75
    MirrorRepeat,
    /// Clamp the value to the border of the texture
    /// Requires the wgpu feature [`Features::ADDRESS_MODE_CLAMP_TO_BORDER`].
    ///
    /// -0.25 -> border
    /// 1.25 -> border
    ClampToBorder,
}

/// Texel mixing mode when sampling between texels.
///
/// This type mirrors [`FilterMode`].
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum ImageFilterMode {
    /// Nearest neighbor sampling.
    ///
    /// This creates a pixelated effect when used as a mag filter.
    #[default]
    Nearest,
    /// Linear Interpolation.
    ///
    /// This makes textures smooth but blurry when used as a mag filter.
    Linear,
}

/// Comparison function used for depth and stencil operations.
///
/// This type mirrors [`CompareFunction`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ImageCompareFunction {
    /// Function never passes
    Never,
    /// Function passes if new value less than existing value
    Less,
    /// Function passes if new value is equal to existing value. When using
    /// this compare function, make sure to mark your Vertex Shader's `@builtin(position)`
    /// output as `@invariant` to prevent artifacting.
    Equal,
    /// Function passes if new value is less than or equal to existing value
    LessEqual,
    /// Function passes if new value is greater than existing value
    Greater,
    /// Function passes if new value is not equal to existing value. When using
    /// this compare function, make sure to mark your Vertex Shader's `@builtin(position)`
    /// output as `@invariant` to prevent artifacting.
    NotEqual,
    /// Function passes if new value is greater than or equal to existing value
    GreaterEqual,
    /// Function always passes
    Always,
}

/// Color variation to use when the sampler addressing mode is [`ImageAddressMode::ClampToBorder`].
///
/// This type mirrors [`SamplerBorderColor`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ImageSamplerBorderColor {
    /// RGBA color `[0, 0, 0, 0]`.
    TransparentBlack,
    /// RGBA color `[0, 0, 0, 1]`.
    OpaqueBlack,
    /// RGBA color `[1, 1, 1, 1]`.
    OpaqueWhite,
    /// On the Metal wgpu backend, this is equivalent to [`Self::TransparentBlack`] for
    /// textures that have an alpha component, and equivalent to [`Self::OpaqueBlack`]
    /// for textures that do not have an alpha component. On other backends,
    /// this is equivalent to [`Self::TransparentBlack`]. Requires
    /// [`Features::ADDRESS_MODE_CLAMP_TO_ZERO`]. Not supported on the web.
    Zero,
}

/// Indicates to an `ImageLoader` how an [`Image`] should be sampled.
///
/// As this type is part of the `ImageLoaderSettings`,
/// it will be serialized to an image asset `.meta` file which might require a migration in case of
/// a breaking change.
///
/// This types mirrors [`SamplerDescriptor`], but that might change in future versions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageSamplerDescriptor {
    pub label: Option<String>,
    /// How to deal with out of bounds accesses in the u (i.e. x) direction.
    pub address_mode_u: ImageAddressMode,
    /// How to deal with out of bounds accesses in the v (i.e. y) direction.
    pub address_mode_v: ImageAddressMode,
    /// How to deal with out of bounds accesses in the w (i.e. z) direction.
    pub address_mode_w: ImageAddressMode,
    /// How to filter the texture when it needs to be magnified (made larger).
    pub mag_filter: ImageFilterMode,
    /// How to filter the texture when it needs to be minified (made smaller).
    pub min_filter: ImageFilterMode,
    /// How to filter between mip map levels
    pub mipmap_filter: ImageFilterMode,
    /// Minimum level of detail (i.e. mip level) to use.
    pub lod_min_clamp: f32,
    /// Maximum level of detail (i.e. mip level) to use.
    pub lod_max_clamp: f32,
    /// If this is enabled, this is a comparison sampler using the given comparison function.
    pub compare: Option<ImageCompareFunction>,
    /// Must be at least 1. If this is not 1, all filter modes must be linear.
    pub anisotropy_clamp: u16,
    /// Border color to use when `address_mode` is [`ImageAddressMode::ClampToBorder`].
    pub border_color: Option<ImageSamplerBorderColor>,
}

impl Default for ImageSamplerDescriptor {
    fn default() -> Self {
        Self {
            address_mode_u: Default::default(),
            address_mode_v: Default::default(),
            address_mode_w: Default::default(),
            mag_filter: Default::default(),
            min_filter: Default::default(),
            mipmap_filter: Default::default(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
            label: None,
        }
    }
}

impl ImageSamplerDescriptor {
    /// Returns a sampler descriptor with [`Linear`](ImageFilterMode::Linear) min and mag filters
    #[inline]
    pub fn linear() -> ImageSamplerDescriptor {
        ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            ..Default::default()
        }
    }

    /// Returns a sampler descriptor with [`Nearest`](ImageFilterMode::Nearest) min and mag filters
    #[inline]
    pub fn nearest() -> ImageSamplerDescriptor {
        ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            mipmap_filter: ImageFilterMode::Nearest,
            ..Default::default()
        }
    }

    pub fn as_wgpu(&self) -> SamplerDescriptor {
        SamplerDescriptor {
            label: self.label.as_deref(),
            address_mode_u: self.address_mode_u.into(),
            address_mode_v: self.address_mode_v.into(),
            address_mode_w: self.address_mode_w.into(),
            mag_filter: self.mag_filter.into(),
            min_filter: self.min_filter.into(),
            mipmap_filter: self.mipmap_filter.into(),
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare.map(Into::into),
            anisotropy_clamp: self.anisotropy_clamp,
            border_color: self.border_color.map(Into::into),
        }
    }
}

impl From<ImageAddressMode> for AddressMode {
    fn from(value: ImageAddressMode) -> Self {
        match value {
            ImageAddressMode::ClampToEdge => AddressMode::ClampToEdge,
            ImageAddressMode::Repeat => AddressMode::Repeat,
            ImageAddressMode::MirrorRepeat => AddressMode::MirrorRepeat,
            ImageAddressMode::ClampToBorder => AddressMode::ClampToBorder,
        }
    }
}

impl From<ImageFilterMode> for FilterMode {
    fn from(value: ImageFilterMode) -> Self {
        match value {
            ImageFilterMode::Nearest => FilterMode::Nearest,
            ImageFilterMode::Linear => FilterMode::Linear,
        }
    }
}

impl From<ImageCompareFunction> for CompareFunction {
    fn from(value: ImageCompareFunction) -> Self {
        match value {
            ImageCompareFunction::Never => CompareFunction::Never,
            ImageCompareFunction::Less => CompareFunction::Less,
            ImageCompareFunction::Equal => CompareFunction::Equal,
            ImageCompareFunction::LessEqual => CompareFunction::LessEqual,
            ImageCompareFunction::Greater => CompareFunction::Greater,
            ImageCompareFunction::NotEqual => CompareFunction::NotEqual,
            ImageCompareFunction::GreaterEqual => CompareFunction::GreaterEqual,
            ImageCompareFunction::Always => CompareFunction::Always,
        }
    }
}

impl From<ImageSamplerBorderColor> for SamplerBorderColor {
    fn from(value: ImageSamplerBorderColor) -> Self {
        match value {
            ImageSamplerBorderColor::TransparentBlack => SamplerBorderColor::TransparentBlack,
            ImageSamplerBorderColor::OpaqueBlack => SamplerBorderColor::OpaqueBlack,
            ImageSamplerBorderColor::OpaqueWhite => SamplerBorderColor::OpaqueWhite,
            ImageSamplerBorderColor::Zero => SamplerBorderColor::Zero,
        }
    }
}

impl From<AddressMode> for ImageAddressMode {
    fn from(value: AddressMode) -> Self {
        match value {
            AddressMode::ClampToEdge => ImageAddressMode::ClampToEdge,
            AddressMode::Repeat => ImageAddressMode::Repeat,
            AddressMode::MirrorRepeat => ImageAddressMode::MirrorRepeat,
            AddressMode::ClampToBorder => ImageAddressMode::ClampToBorder,
        }
    }
}

impl From<FilterMode> for ImageFilterMode {
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Nearest => ImageFilterMode::Nearest,
            FilterMode::Linear => ImageFilterMode::Linear,
        }
    }
}

impl From<CompareFunction> for ImageCompareFunction {
    fn from(value: CompareFunction) -> Self {
        match value {
            CompareFunction::Never => ImageCompareFunction::Never,
            CompareFunction::Less => ImageCompareFunction::Less,
            CompareFunction::Equal => ImageCompareFunction::Equal,
            CompareFunction::LessEqual => ImageCompareFunction::LessEqual,
            CompareFunction::Greater => ImageCompareFunction::Greater,
            CompareFunction::NotEqual => ImageCompareFunction::NotEqual,
            CompareFunction::GreaterEqual => ImageCompareFunction::GreaterEqual,
            CompareFunction::Always => ImageCompareFunction::Always,
        }
    }
}

impl From<SamplerBorderColor> for ImageSamplerBorderColor {
    fn from(value: SamplerBorderColor) -> Self {
        match value {
            SamplerBorderColor::TransparentBlack => ImageSamplerBorderColor::TransparentBlack,
            SamplerBorderColor::OpaqueBlack => ImageSamplerBorderColor::OpaqueBlack,
            SamplerBorderColor::OpaqueWhite => ImageSamplerBorderColor::OpaqueWhite,
            SamplerBorderColor::Zero => ImageSamplerBorderColor::Zero,
        }
    }
}

impl<'a> From<SamplerDescriptor<'a>> for ImageSamplerDescriptor {
    fn from(value: SamplerDescriptor) -> Self {
        ImageSamplerDescriptor {
            label: value.label.map(ToString::to_string),
            address_mode_u: value.address_mode_u.into(),
            address_mode_v: value.address_mode_v.into(),
            address_mode_w: value.address_mode_w.into(),
            mag_filter: value.mag_filter.into(),
            min_filter: value.min_filter.into(),
            mipmap_filter: value.mipmap_filter.into(),
            lod_min_clamp: value.lod_min_clamp,
            lod_max_clamp: value.lod_max_clamp,
            compare: value.compare.map(Into::into),
            anisotropy_clamp: value.anisotropy_clamp,
            border_color: value.border_color.map(Into::into),
        }
    }
}

impl Default for Image {
    /// default is a 1x1x1 all '1.0' texture
    fn default() -> Self {
        let format = TextureFormat::bevy_default();
        let data = vec![255; format.pixel_size()];
        Image {
            data,
            texture_descriptor: TextureDescriptor {
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                format,
                dimension: TextureDimension::D2,
                label: None,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            sampler: ImageSampler::Default,
            texture_view_descriptor: None,
            asset_usage: RenderAssetUsages::default(),
        }
    }
}

impl Image {
    /// Creates a new image from raw binary data and the corresponding metadata.
    ///
    /// # Panics
    /// Panics if the length of the `data`, volume of the `size` and the size of the `format`
    /// do not match.
    pub fn new(
        size: Extent3d,
        dimension: TextureDimension,
        data: Vec<u8>,
        format: TextureFormat,
        asset_usage: RenderAssetUsages,
    ) -> Self {
        debug_assert_eq!(
            size.volume() * format.pixel_size(),
            data.len(),
            "Pixel data, size and format have to match",
        );
        let mut image = Self {
            data,
            ..Default::default()
        };
        image.texture_descriptor.dimension = dimension;
        image.texture_descriptor.size = size;
        image.texture_descriptor.format = format;
        image.asset_usage = asset_usage;
        image
    }

    /// A transparent white 1x1x1 image.
    ///
    /// Contrast to [`Image::default`], which is opaque.
    pub fn transparent() -> Image {
        // We rely on the default texture format being RGBA8UnormSrgb
        // when constructing a transparent color from bytes.
        // If this changes, this function will need to be updated.
        let format = TextureFormat::bevy_default();
        debug_assert!(format.pixel_size() == 4);
        let data = vec![255, 255, 255, 0];
        Image {
            data,
            texture_descriptor: TextureDescriptor {
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                format,
                dimension: TextureDimension::D2,
                label: None,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            sampler: ImageSampler::Default,
            texture_view_descriptor: None,
            asset_usage: RenderAssetUsages::default(),
        }
    }

    /// Creates a new image from raw binary data and the corresponding metadata, by filling
    /// the image data with the `pixel` data repeated multiple times.
    ///
    /// # Panics
    /// Panics if the size of the `format` is not a multiple of the length of the `pixel` data.
    pub fn new_fill(
        size: Extent3d,
        dimension: TextureDimension,
        pixel: &[u8],
        format: TextureFormat,
        asset_usage: RenderAssetUsages,
    ) -> Self {
        let mut value = Image::default();
        value.texture_descriptor.format = format;
        value.texture_descriptor.dimension = dimension;
        value.asset_usage = asset_usage;
        value.resize(size);

        debug_assert_eq!(
            pixel.len() % format.pixel_size(),
            0,
            "Must not have incomplete pixel data (pixel size is {}B).",
            format.pixel_size(),
        );
        debug_assert!(
            pixel.len() <= value.data.len(),
            "Fill data must fit within pixel buffer (expected {}B).",
            value.data.len(),
        );

        for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
            current_pixel.copy_from_slice(pixel);
        }
        value
    }

    /// Returns the width of a 2D image.
    #[inline]
    pub fn width(&self) -> u32 {
        self.texture_descriptor.size.width
    }

    /// Returns the height of a 2D image.
    #[inline]
    pub fn height(&self) -> u32 {
        self.texture_descriptor.size.height
    }

    /// Returns the aspect ratio (width / height) of a 2D image.
    #[inline]
    pub fn aspect_ratio(&self) -> AspectRatio {
        AspectRatio::try_from_pixels(self.width(), self.height()).expect(
            "Failed to calculate aspect ratio: Image dimensions must be positive, non-zero values",
        )
    }

    /// Returns the size of a 2D image as f32.
    #[inline]
    pub fn size_f32(&self) -> Vec2 {
        Vec2::new(self.width() as f32, self.height() as f32)
    }

    /// Returns the size of a 2D image.
    #[inline]
    pub fn size(&self) -> UVec2 {
        UVec2::new(self.width(), self.height())
    }

    /// Resizes the image to the new size, by removing information or appending 0 to the `data`.
    /// Does not properly resize the contents of the image, but only its internal `data` buffer.
    pub fn resize(&mut self, size: Extent3d) {
        self.texture_descriptor.size = size;
        self.data.resize(
            size.volume() * self.texture_descriptor.format.pixel_size(),
            0,
        );
    }

    /// Changes the `size`, asserting that the total number of data elements (pixels) remains the
    /// same.
    ///
    /// # Panics
    /// Panics if the `new_size` does not have the same volume as to old one.
    pub fn reinterpret_size(&mut self, new_size: Extent3d) {
        assert_eq!(
            new_size.volume(),
            self.texture_descriptor.size.volume(),
            "Incompatible sizes: old = {:?} new = {:?}",
            self.texture_descriptor.size,
            new_size
        );

        self.texture_descriptor.size = new_size;
    }

    /// Takes a 2D image containing vertically stacked images of the same size, and reinterprets
    /// it as a 2D array texture, where each of the stacked images becomes one layer of the
    /// array. This is primarily for use with the `texture2DArray` shader uniform type.
    ///
    /// # Panics
    /// Panics if the texture is not 2D, has more than one layers or is not evenly dividable into
    /// the `layers`.
    pub fn reinterpret_stacked_2d_as_array(&mut self, layers: u32) {
        // Must be a stacked image, and the height must be divisible by layers.
        assert_eq!(self.texture_descriptor.dimension, TextureDimension::D2);
        assert_eq!(self.texture_descriptor.size.depth_or_array_layers, 1);
        assert_eq!(self.height() % layers, 0);

        self.reinterpret_size(Extent3d {
            width: self.width(),
            height: self.height() / layers,
            depth_or_array_layers: layers,
        });
    }

    /// Convert a texture from a format to another. Only a few formats are
    /// supported as input and output:
    /// - `TextureFormat::R8Unorm`
    /// - `TextureFormat::Rg8Unorm`
    /// - `TextureFormat::Rgba8UnormSrgb`
    ///
    /// To get [`Image`] as a [`image::DynamicImage`] see:
    /// [`Image::try_into_dynamic`].
    pub fn convert(&self, new_format: TextureFormat) -> Option<Self> {
        self.clone()
            .try_into_dynamic()
            .ok()
            .and_then(|img| match new_format {
                TextureFormat::R8Unorm => {
                    Some((image::DynamicImage::ImageLuma8(img.into_luma8()), false))
                }
                TextureFormat::Rg8Unorm => Some((
                    image::DynamicImage::ImageLumaA8(img.into_luma_alpha8()),
                    false,
                )),
                TextureFormat::Rgba8UnormSrgb => {
                    Some((image::DynamicImage::ImageRgba8(img.into_rgba8()), true))
                }
                _ => None,
            })
            .map(|(dyn_img, is_srgb)| Self::from_dynamic(dyn_img, is_srgb, self.asset_usage))
    }

    /// Load a bytes buffer in a [`Image`], according to type `image_type`, using the `image`
    /// crate
    pub fn from_buffer(
        #[cfg(all(debug_assertions, feature = "dds"))] name: String,
        buffer: &[u8],
        image_type: ImageType,
        #[expect(
            clippy::allow_attributes,
            reason = "`unused_variables` may not always lint"
        )]
        #[allow(
            unused_variables,
            reason = "`supported_compressed_formats` is needed where the image format is `Basis`, `Dds`, or `Ktx2`; if these are disabled, then `supported_compressed_formats` is unused."
        )]
        supported_compressed_formats: CompressedImageFormats,
        is_srgb: bool,
        image_sampler: ImageSampler,
        asset_usage: RenderAssetUsages,
    ) -> Result<Image, TextureError> {
        let format = image_type.to_image_format()?;

        // Load the image in the expected format.
        // Some formats like PNG allow for R or RG textures too, so the texture
        // format needs to be determined. For RGB textures an alpha channel
        // needs to be added, so the image data needs to be converted in those
        // cases.

        let mut image = match format {
            #[cfg(feature = "basis-universal")]
            ImageFormat::Basis => {
                basis_buffer_to_image(buffer, supported_compressed_formats, is_srgb)?
            }
            #[cfg(feature = "dds")]
            ImageFormat::Dds => dds_buffer_to_image(
                #[cfg(debug_assertions)]
                name,
                buffer,
                supported_compressed_formats,
                is_srgb,
            )?,
            #[cfg(feature = "ktx2")]
            ImageFormat::Ktx2 => {
                ktx2_buffer_to_image(buffer, supported_compressed_formats, is_srgb)?
            }
            #[expect(
                clippy::allow_attributes,
                reason = "`unreachable_patterns` may not always lint"
            )]
            #[allow(
                unreachable_patterns,
                reason = "The wildcard pattern may be unreachable if only the specially-handled formats are enabled; however, the wildcard pattern is needed for any formats not specially handled"
            )]
            _ => {
                let image_crate_format = format
                    .as_image_crate_format()
                    .ok_or_else(|| TextureError::UnsupportedTextureFormat(format!("{format:?}")))?;
                let mut reader = image::ImageReader::new(std::io::Cursor::new(buffer));
                reader.set_format(image_crate_format);
                reader.no_limits();
                let dyn_img = reader.decode()?;
                Self::from_dynamic(dyn_img, is_srgb, asset_usage)
            }
        };
        image.sampler = image_sampler;
        Ok(image)
    }

    /// Whether the texture format is compressed or uncompressed
    pub fn is_compressed(&self) -> bool {
        let format_description = self.texture_descriptor.format;
        format_description
            .required_features()
            .contains(Features::TEXTURE_COMPRESSION_ASTC)
            || format_description
                .required_features()
                .contains(Features::TEXTURE_COMPRESSION_BC)
            || format_description
                .required_features()
                .contains(Features::TEXTURE_COMPRESSION_ETC2)
    }

    /// Compute the byte offset where the data of a specific pixel is stored
    ///
    /// Returns None if the provided coordinates are out of bounds.
    ///
    /// For 2D textures, Z is the layer number. For 1D textures, Y and Z are ignored.
    #[inline(always)]
    pub fn pixel_data_offset(&self, coords: UVec3) -> Option<usize> {
        let width = self.texture_descriptor.size.width;
        let height = self.texture_descriptor.size.height;
        let depth = self.texture_descriptor.size.depth_or_array_layers;

        let pixel_size = self.texture_descriptor.format.pixel_size();
        let pixel_offset = match self.texture_descriptor.dimension {
            TextureDimension::D3 | TextureDimension::D2 => {
                if coords.x >= width || coords.y >= height || coords.z >= depth {
                    return None;
                }
                coords.z * height * width + coords.y * width + coords.x
            }
            TextureDimension::D1 => {
                if coords.x >= width {
                    return None;
                }
                coords.x
            }
        };

        Some(pixel_offset as usize * pixel_size)
    }

    /// Get a reference to the data bytes where a specific pixel's value is stored
    #[inline(always)]
    pub fn pixel_bytes(&self, coords: UVec3) -> Option<&[u8]> {
        let len = self.texture_descriptor.format.pixel_size();
        self.pixel_data_offset(coords)
            .map(|start| &self.data[start..(start + len)])
    }

    /// Get a mutable reference to the data bytes where a specific pixel's value is stored
    #[inline(always)]
    pub fn pixel_bytes_mut(&mut self, coords: UVec3) -> Option<&mut [u8]> {
        let len = self.texture_descriptor.format.pixel_size();
        self.pixel_data_offset(coords)
            .map(|start| &mut self.data[start..(start + len)])
    }

    /// Read the color of a specific pixel (1D texture).
    ///
    /// See [`get_color_at`](Self::get_color_at) for more details.
    #[inline(always)]
    pub fn get_color_at_1d(&self, x: u32) -> Result<Color, TextureAccessError> {
        if self.texture_descriptor.dimension != TextureDimension::D1 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.get_color_at_internal(UVec3::new(x, 0, 0))
    }

    /// Read the color of a specific pixel (2D texture).
    ///
    /// This function will find the raw byte data of a specific pixel and
    /// decode it into a user-friendly [`Color`] struct for you.
    ///
    /// Supports many of the common [`TextureFormat`]s:
    ///  - RGBA/BGRA 8-bit unsigned integer, both sRGB and Linear
    ///  - 16-bit and 32-bit unsigned integer
    ///  - 32-bit float
    ///
    /// Be careful: as the data is converted to [`Color`] (which uses `f32` internally),
    /// there may be issues with precision when using non-float [`TextureFormat`]s.
    /// If you read a value you previously wrote using `set_color_at`, it will not match.
    /// If you are working with a 32-bit integer [`TextureFormat`], the value will be
    /// inaccurate (as `f32` does not have enough bits to represent it exactly).
    ///
    /// Single channel (R) formats are assumed to represent grayscale, so the value
    /// will be copied to all three RGB channels in the resulting [`Color`].
    ///
    /// Other [`TextureFormat`]s are unsupported, such as:
    ///  - block-compressed formats
    ///  - non-byte-aligned formats like 10-bit
    ///  - 16-bit float formats
    ///  - signed integer formats
    #[inline(always)]
    pub fn get_color_at(&self, x: u32, y: u32) -> Result<Color, TextureAccessError> {
        if self.texture_descriptor.dimension != TextureDimension::D2 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.get_color_at_internal(UVec3::new(x, y, 0))
    }

    /// Read the color of a specific pixel (2D texture with layers or 3D texture).
    ///
    /// See [`get_color_at`](Self::get_color_at) for more details.
    #[inline(always)]
    pub fn get_color_at_3d(&self, x: u32, y: u32, z: u32) -> Result<Color, TextureAccessError> {
        match (
            self.texture_descriptor.dimension,
            self.texture_descriptor.size.depth_or_array_layers,
        ) {
            (TextureDimension::D3, _) | (TextureDimension::D2, 2..) => {
                self.get_color_at_internal(UVec3::new(x, y, z))
            }
            _ => Err(TextureAccessError::WrongDimension),
        }
    }

    /// Change the color of a specific pixel (1D texture).
    ///
    /// See [`set_color_at`](Self::set_color_at) for more details.
    #[inline(always)]
    pub fn set_color_at_1d(&mut self, x: u32, color: Color) -> Result<(), TextureAccessError> {
        if self.texture_descriptor.dimension != TextureDimension::D1 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.set_color_at_internal(UVec3::new(x, 0, 0), color)
    }

    /// Change the color of a specific pixel (2D texture).
    ///
    /// This function will find the raw byte data of a specific pixel and
    /// change it according to a [`Color`] you provide. The [`Color`] struct
    /// will be encoded into the [`Image`]'s [`TextureFormat`].
    ///
    /// Supports many of the common [`TextureFormat`]s:
    ///  - RGBA/BGRA 8-bit unsigned integer, both sRGB and Linear
    ///  - 16-bit and 32-bit unsigned integer (with possibly-limited precision, as [`Color`] uses `f32`)
    ///  - 32-bit float
    ///
    /// Be careful: writing to non-float [`TextureFormat`]s is lossy! The data has to be converted,
    /// so if you read it back using `get_color_at`, the `Color` you get will not equal the value
    /// you used when writing it using this function.
    ///
    /// For R and RG formats, only the respective values from the linear RGB [`Color`] will be used.
    ///
    /// Other [`TextureFormat`]s are unsupported, such as:
    ///  - block-compressed formats
    ///  - non-byte-aligned formats like 10-bit
    ///  - 16-bit float formats
    ///  - signed integer formats
    #[inline(always)]
    pub fn set_color_at(&mut self, x: u32, y: u32, color: Color) -> Result<(), TextureAccessError> {
        if self.texture_descriptor.dimension != TextureDimension::D2 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.set_color_at_internal(UVec3::new(x, y, 0), color)
    }

    /// Change the color of a specific pixel (2D texture with layers or 3D texture).
    ///
    /// See [`set_color_at`](Self::set_color_at) for more details.
    #[inline(always)]
    pub fn set_color_at_3d(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        color: Color,
    ) -> Result<(), TextureAccessError> {
        match (
            self.texture_descriptor.dimension,
            self.texture_descriptor.size.depth_or_array_layers,
        ) {
            (TextureDimension::D3, _) | (TextureDimension::D2, 2..) => {
                self.set_color_at_internal(UVec3::new(x, y, z), color)
            }
            _ => Err(TextureAccessError::WrongDimension),
        }
    }

    #[inline(always)]
    fn get_color_at_internal(&self, coords: UVec3) -> Result<Color, TextureAccessError> {
        let Some(bytes) = self.pixel_bytes(coords) else {
            return Err(TextureAccessError::OutOfBounds {
                x: coords.x,
                y: coords.y,
                z: coords.z,
            });
        };

        // NOTE: GPUs are always Little Endian.
        // Make sure to respect that when we create color values from bytes.
        match self.texture_descriptor.format {
            TextureFormat::Rgba8UnormSrgb => Ok(Color::srgba(
                bytes[0] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[2] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8Uint => Ok(Color::linear_rgba(
                bytes[0] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[2] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Bgra8UnormSrgb => Ok(Color::srgba(
                bytes[2] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[0] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Bgra8Unorm => Ok(Color::linear_rgba(
                bytes[2] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[0] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Rgba32Float => Ok(Color::linear_rgba(
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            )),
            TextureFormat::Rgba16Unorm | TextureFormat::Rgba16Uint => {
                let (r, g, b, a) = (
                    u16::from_le_bytes([bytes[0], bytes[1]]),
                    u16::from_le_bytes([bytes[2], bytes[3]]),
                    u16::from_le_bytes([bytes[4], bytes[5]]),
                    u16::from_le_bytes([bytes[6], bytes[7]]),
                );
                Ok(Color::linear_rgba(
                    // going via f64 to avoid rounding errors with large numbers and division
                    (r as f64 / u16::MAX as f64) as f32,
                    (g as f64 / u16::MAX as f64) as f32,
                    (b as f64 / u16::MAX as f64) as f32,
                    (a as f64 / u16::MAX as f64) as f32,
                ))
            }
            TextureFormat::Rgba32Uint => {
                let (r, g, b, a) = (
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                    u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                    u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
                );
                Ok(Color::linear_rgba(
                    // going via f64 to avoid rounding errors with large numbers and division
                    (r as f64 / u32::MAX as f64) as f32,
                    (g as f64 / u32::MAX as f64) as f32,
                    (b as f64 / u32::MAX as f64) as f32,
                    (a as f64 / u32::MAX as f64) as f32,
                ))
            }
            // assume R-only texture format means grayscale (linear)
            // copy value to all of RGB in Color
            TextureFormat::R8Unorm | TextureFormat::R8Uint => {
                let x = bytes[0] as f32 / u8::MAX as f32;
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R16Unorm | TextureFormat::R16Uint => {
                let x = u16::from_le_bytes([bytes[0], bytes[1]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let x = (x as f64 / u16::MAX as f64) as f32;
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R32Uint => {
                let x = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let x = (x as f64 / u32::MAX as f64) as f32;
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R32Float => {
                let x = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::Rg8Unorm | TextureFormat::Rg8Uint => {
                let r = bytes[0] as f32 / u8::MAX as f32;
                let g = bytes[1] as f32 / u8::MAX as f32;
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg16Unorm | TextureFormat::Rg16Uint => {
                let r = u16::from_le_bytes([bytes[0], bytes[1]]);
                let g = u16::from_le_bytes([bytes[2], bytes[3]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let r = (r as f64 / u16::MAX as f64) as f32;
                let g = (g as f64 / u16::MAX as f64) as f32;
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg32Uint => {
                let r = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let g = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let r = (r as f64 / u32::MAX as f64) as f32;
                let g = (g as f64 / u32::MAX as f64) as f32;
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg32Float => {
                let r = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let g = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            _ => Err(TextureAccessError::UnsupportedTextureFormat(
                self.texture_descriptor.format,
            )),
        }
    }

    #[inline(always)]
    fn set_color_at_internal(
        &mut self,
        coords: UVec3,
        color: Color,
    ) -> Result<(), TextureAccessError> {
        let format = self.texture_descriptor.format;

        let Some(bytes) = self.pixel_bytes_mut(coords) else {
            return Err(TextureAccessError::OutOfBounds {
                x: coords.x,
                y: coords.y,
                z: coords.z,
            });
        };

        // NOTE: GPUs are always Little Endian.
        // Make sure to respect that when we convert color values to bytes.
        match format {
            TextureFormat::Rgba8UnormSrgb => {
                let [r, g, b, a] = Srgba::from(color).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (b * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8Uint => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (b * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Bgra8UnormSrgb => {
                let [r, g, b, a] = Srgba::from(color).to_f32_array();
                bytes[0] = (b * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (r * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Bgra8Unorm => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0] = (b * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (r * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Rgba32Float => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0..4].copy_from_slice(&f32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&f32::to_le_bytes(g));
                bytes[8..12].copy_from_slice(&f32::to_le_bytes(b));
                bytes[12..16].copy_from_slice(&f32::to_le_bytes(a));
            }
            TextureFormat::Rgba16Unorm | TextureFormat::Rgba16Uint => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                let [r, g, b, a] = [
                    (r * u16::MAX as f32) as u16,
                    (g * u16::MAX as f32) as u16,
                    (b * u16::MAX as f32) as u16,
                    (a * u16::MAX as f32) as u16,
                ];
                bytes[0..2].copy_from_slice(&u16::to_le_bytes(r));
                bytes[2..4].copy_from_slice(&u16::to_le_bytes(g));
                bytes[4..6].copy_from_slice(&u16::to_le_bytes(b));
                bytes[6..8].copy_from_slice(&u16::to_le_bytes(a));
            }
            TextureFormat::Rgba32Uint => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                let [r, g, b, a] = [
                    (r * u32::MAX as f32) as u32,
                    (g * u32::MAX as f32) as u32,
                    (b * u32::MAX as f32) as u32,
                    (a * u32::MAX as f32) as u32,
                ];
                bytes[0..4].copy_from_slice(&u32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&u32::to_le_bytes(g));
                bytes[8..12].copy_from_slice(&u32::to_le_bytes(b));
                bytes[12..16].copy_from_slice(&u32::to_le_bytes(a));
            }
            TextureFormat::R8Unorm | TextureFormat::R8Uint => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
            }
            TextureFormat::R16Unorm | TextureFormat::R16Uint => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                let r = (r * u16::MAX as f32) as u16;
                bytes[0..2].copy_from_slice(&u16::to_le_bytes(r));
            }
            TextureFormat::R32Uint => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                // go via f64 to avoid imprecision
                let r = (r as f64 * u32::MAX as f64) as u32;
                bytes[0..4].copy_from_slice(&u32::to_le_bytes(r));
            }
            TextureFormat::R32Float => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                bytes[0..4].copy_from_slice(&f32::to_le_bytes(r));
            }
            TextureFormat::Rg8Unorm | TextureFormat::Rg8Uint => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
            }
            TextureFormat::Rg16Unorm | TextureFormat::Rg16Uint => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                let r = (r * u16::MAX as f32) as u16;
                let g = (g * u16::MAX as f32) as u16;
                bytes[0..2].copy_from_slice(&u16::to_le_bytes(r));
                bytes[2..4].copy_from_slice(&u16::to_le_bytes(g));
            }
            TextureFormat::Rg32Uint => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                // go via f64 to avoid imprecision
                let r = (r as f64 * u32::MAX as f64) as u32;
                let g = (g as f64 * u32::MAX as f64) as u32;
                bytes[0..4].copy_from_slice(&u32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&u32::to_le_bytes(g));
            }
            TextureFormat::Rg32Float => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                bytes[0..4].copy_from_slice(&f32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&f32::to_le_bytes(g));
            }
            _ => {
                return Err(TextureAccessError::UnsupportedTextureFormat(
                    self.texture_descriptor.format,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DataFormat {
    Rgb,
    Rgba,
    Rrr,
    Rrrg,
    Rg,
}

#[derive(Clone, Copy, Debug)]
pub enum TranscodeFormat {
    Etc1s,
    Uastc(DataFormat),
    // Has to be transcoded to R8Unorm for use with `wgpu`
    R8UnormSrgb,
    // Has to be transcoded to R8G8Unorm for use with `wgpu`
    Rg8UnormSrgb,
    // Has to be transcoded to Rgba8 for use with `wgpu`
    Rgb8,
}

/// An error that occurs when accessing specific pixels in a texture
#[derive(Error, Debug)]
pub enum TextureAccessError {
    #[error("out of bounds (x: {x}, y: {y}, z: {z})")]
    OutOfBounds { x: u32, y: u32, z: u32 },
    #[error("unsupported texture format: {0:?}")]
    UnsupportedTextureFormat(TextureFormat),
    #[error("attempt to access texture with different dimension")]
    WrongDimension,
}

/// An error that occurs when loading a texture
#[derive(Error, Debug)]
pub enum TextureError {
    #[error("invalid image mime type: {0}")]
    InvalidImageMimeType(String),
    #[error("invalid image extension: {0}")]
    InvalidImageExtension(String),
    #[error("failed to load an image: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("unsupported texture format: {0}")]
    UnsupportedTextureFormat(String),
    #[error("supercompression not supported: {0}")]
    SuperCompressionNotSupported(String),
    #[error("failed to load an image: {0}")]
    SuperDecompressionError(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("transcode error: {0}")]
    TranscodeError(String),
    #[error("format requires transcoding: {0:?}")]
    FormatRequiresTranscodingError(TranscodeFormat),
    /// Only cubemaps with six faces are supported.
    #[error("only cubemaps with six faces are supported")]
    IncompleteCubemap,
}

/// The type of a raw image buffer.
#[derive(Debug)]
pub enum ImageType<'a> {
    /// The mime type of an image, for example `"image/png"`.
    MimeType(&'a str),
    /// The extension of an image file, for example `"png"`.
    Extension(&'a str),
    /// The direct format of the image
    Format(ImageFormat),
}

impl<'a> ImageType<'a> {
    pub fn to_image_format(&self) -> Result<ImageFormat, TextureError> {
        match self {
            ImageType::MimeType(mime_type) => ImageFormat::from_mime_type(mime_type)
                .ok_or_else(|| TextureError::InvalidImageMimeType(mime_type.to_string())),
            ImageType::Extension(extension) => ImageFormat::from_extension(extension)
                .ok_or_else(|| TextureError::InvalidImageExtension(extension.to_string())),
            ImageType::Format(format) => Ok(*format),
        }
    }
}

/// Used to calculate the volume of an item.
pub trait Volume {
    fn volume(&self) -> usize;
}

impl Volume for Extent3d {
    /// Calculates the volume of the [`Extent3d`].
    fn volume(&self) -> usize {
        (self.width * self.height * self.depth_or_array_layers) as usize
    }
}

/// Extends the wgpu [`TextureFormat`] with information about the pixel.
pub trait TextureFormatPixelInfo {
    /// Returns the size of a pixel in bytes of the format.
    fn pixel_size(&self) -> usize;
}

impl TextureFormatPixelInfo for TextureFormat {
    fn pixel_size(&self) -> usize {
        let info = self;
        match info.block_dimensions() {
            (1, 1) => info.block_copy_size(None).unwrap() as usize,
            _ => panic!("Using pixel_size for compressed textures is invalid"),
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
    #[repr(transparent)]
    pub struct CompressedImageFormats: u32 {
        const NONE     = 0;
        const ASTC_LDR = 1 << 0;
        const BC       = 1 << 1;
        const ETC2     = 1 << 2;
    }
}

impl CompressedImageFormats {
    pub fn from_features(features: Features) -> Self {
        let mut supported_compressed_formats = Self::default();
        if features.contains(Features::TEXTURE_COMPRESSION_ASTC) {
            supported_compressed_formats |= Self::ASTC_LDR;
        }
        if features.contains(Features::TEXTURE_COMPRESSION_BC) {
            supported_compressed_formats |= Self::BC;
        }
        if features.contains(Features::TEXTURE_COMPRESSION_ETC2) {
            supported_compressed_formats |= Self::ETC2;
        }
        supported_compressed_formats
    }

    pub fn supports(&self, format: TextureFormat) -> bool {
        match format {
            TextureFormat::Bc1RgbaUnorm
            | TextureFormat::Bc1RgbaUnormSrgb
            | TextureFormat::Bc2RgbaUnorm
            | TextureFormat::Bc2RgbaUnormSrgb
            | TextureFormat::Bc3RgbaUnorm
            | TextureFormat::Bc3RgbaUnormSrgb
            | TextureFormat::Bc4RUnorm
            | TextureFormat::Bc4RSnorm
            | TextureFormat::Bc5RgUnorm
            | TextureFormat::Bc5RgSnorm
            | TextureFormat::Bc6hRgbUfloat
            | TextureFormat::Bc6hRgbFloat
            | TextureFormat::Bc7RgbaUnorm
            | TextureFormat::Bc7RgbaUnormSrgb => self.contains(CompressedImageFormats::BC),
            TextureFormat::Etc2Rgb8Unorm
            | TextureFormat::Etc2Rgb8UnormSrgb
            | TextureFormat::Etc2Rgb8A1Unorm
            | TextureFormat::Etc2Rgb8A1UnormSrgb
            | TextureFormat::Etc2Rgba8Unorm
            | TextureFormat::Etc2Rgba8UnormSrgb
            | TextureFormat::EacR11Unorm
            | TextureFormat::EacR11Snorm
            | TextureFormat::EacRg11Unorm
            | TextureFormat::EacRg11Snorm => self.contains(CompressedImageFormats::ETC2),
            TextureFormat::Astc { .. } => self.contains(CompressedImageFormats::ASTC_LDR),
            _ => true,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn image_size() {
        let size = Extent3d {
            width: 200,
            height: 100,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD,
        );
        assert_eq!(
            Vec2::new(size.width as f32, size.height as f32),
            image.size_f32()
        );
    }

    #[test]
    fn image_default_size() {
        let image = Image::default();
        assert_eq!(UVec2::ONE, image.size());
        assert_eq!(Vec2::ONE, image.size_f32());
    }

    #[test]
    fn on_edge_pixel_is_invalid() {
        let image = Image::new_fill(
            Extent3d {
                width: 5,
                height: 10,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD,
        );
        assert!(matches!(image.get_color_at(4, 9), Ok(Color::BLACK)));
        assert!(matches!(
            image.get_color_at(0, 10),
            Err(TextureAccessError::OutOfBounds { x: 0, y: 10, z: 0 })
        ));
        assert!(matches!(
            image.get_color_at(5, 10),
            Err(TextureAccessError::OutOfBounds { x: 5, y: 10, z: 0 })
        ));
    }

    #[test]
    fn get_set_pixel_2d_with_layers() {
        let mut image = Image::new_fill(
            Extent3d {
                width: 5,
                height: 10,
                depth_or_array_layers: 3,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD,
        );
        image.set_color_at_3d(0, 0, 0, Color::WHITE).unwrap();
        assert!(matches!(image.get_color_at_3d(0, 0, 0), Ok(Color::WHITE)));
        image.set_color_at_3d(2, 3, 1, Color::WHITE).unwrap();
        assert!(matches!(image.get_color_at_3d(2, 3, 1), Ok(Color::WHITE)));
        image.set_color_at_3d(4, 9, 2, Color::WHITE).unwrap();
        assert!(matches!(image.get_color_at_3d(4, 9, 2), Ok(Color::WHITE)));
    }
}
