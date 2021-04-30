use super::{
    image_texture_conversion::image_to_texture, Extent3d, SamplerDescriptor, TextureDescriptor,
    TextureDimension, TextureFormat,
};
use crate::renderer::{
    RenderResource, RenderResourceContext, RenderResourceId, RenderResourceType,
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{event::EventReader, system::Res};
use bevy_reflect::TypeUuid;
use bevy_utils::HashSet;
use thiserror::Error;

/// Reexport to allow users to import it via Bevy and not require depending on `image`
/// Rename it to avoid confusion with GPU texture sampler filtering modes.
pub use image::imageops::FilterType as ImageResizeFilter;

pub const TEXTURE_ASSET_INDEX: u64 = 0;
pub const SAMPLER_ASSET_INDEX: u64 = 1;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f"]
pub struct Texture {
    /// Data of the texture's main image (base mipmap level)
    pub data: Vec<u8>,
    /// Data of any additional mipmap levels
    pub mipmaps: Vec<Vec<u8>>,
    pub size: Extent3d,
    /// How many mipmap levels to copy into the GPU
    pub max_mip_level: Option<usize>,
    pub format: TextureFormat,
    pub dimension: TextureDimension,
    pub sampler: SamplerDescriptor,
}

impl Default for Texture {
    fn default() -> Self {
        Texture {
            data: Vec::new(),
            mipmaps: Vec::new(),
            size: Extent3d {
                width: 1,
                height: 1,
                depth: 1,
            },
            max_mip_level: None,
            format: TextureFormat::Rgba8UnormSrgb,
            dimension: TextureDimension::D2,
            sampler: Default::default(),
        }
    }
}

impl Texture {
    /// Create a new texture without mipmaps
    pub fn new(
        size: Extent3d,
        dimension: TextureDimension,
        data: Vec<u8>,
        format: TextureFormat,
    ) -> Self {
        debug_assert_eq!(
            size.volume() * format.pixel_size(),
            data.len(),
            "Pixel data, size and format have to match",
        );
        Self {
            data,
            size,
            dimension,
            format,
            ..Default::default()
        }
    }

    pub fn new_fill(
        size: Extent3d,
        dimension: TextureDimension,
        pixel: &[u8],
        format: TextureFormat,
    ) -> Self {
        let mut value = Texture {
            format,
            dimension,
            ..Default::default()
        };
        value.resize(size);

        debug_assert_eq!(
            pixel.len() % format.pixel_size(),
            0,
            "Must not have incomplete pixel data."
        );
        debug_assert!(
            pixel.len() <= value.data.len(),
            "Fill data must fit within pixel buffer."
        );

        for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
            current_pixel.copy_from_slice(&pixel);
        }
        value
    }

    pub fn aspect_2d(&self) -> f32 {
        self.size.height as f32 / self.size.width as f32
    }

    pub fn mip_size(&self, level: usize) -> Extent3d {
        Extent3d {
            width: (self.size.width >> level).max(1),
            height: (self.size.height >> level).max(1),
            depth: (self.size.depth >> level).max(1),
        }
    }

    pub fn resize(&mut self, size: Extent3d) {
        self.size = size;
        self.data
            .resize(size.volume() * self.format.pixel_size(), 0);

        // resize mipmaps
        for level in 0..self.mipmaps.len() {
            let mip_volume = self.mip_size(level + 1).volume();

            self.mipmaps[level].resize(mip_volume * self.format.pixel_size(), 0);

            if mip_volume == 1 {
                // This was the last mip level, remove any extras
                self.mipmaps.truncate(level + 1);
                self.max_mip_level = self.max_mip_level.map(|old| old.min(level + 1));
                break;
            }
        }
    }

    /// Changes the `size`, asserting that the total number of data elements (pixels) remains the
    /// same.
    pub fn reinterpret_size(&mut self, new_size: Extent3d) {
        assert!(
            new_size.volume() == self.size.volume(),
            "Incompatible sizes: old = {:?} new = {:?}",
            self.size,
            new_size
        );

        self.size = new_size;
    }

    /// Takes a 2D texture containing vertically stacked images of the same size, and reinterprets
    /// it as a 2D array texture, where each of the stacked images becomes one layer of the
    /// array. This is primarily for use with the `texture2DArray` shader uniform type.
    pub fn reinterpret_stacked_2d_as_array(&mut self, layers: u32) {
        // Must be a stacked image, and the height must be divisible by layers.
        assert!(self.dimension == TextureDimension::D2);
        assert!(self.size.depth == 1);
        assert_eq!(self.size.height % layers, 0);

        self.reinterpret_size(Extent3d {
            width: self.size.width,
            height: self.size.height / layers,
            depth: layers,
        });
    }

    /// Iterator that gives the texel data for each mipmap level to use in rendering
    pub(crate) fn iter_mipmaps(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(self.data.as_slice()).chain(self.mipmaps.iter().map(|vec| vec.as_slice()))
    }

    /// Generate mipmap images by downscaling the texture
    ///
    /// Currently this operation is performed on the CPU.
    ///
    /// The number of mipmaps to generate is limited to `max_mip_level`, or
    /// until either width or height reaches `min_size`.
    ///
    /// Mipmaps are generated by downscaling the base texture image.
    ///
    /// This method uses the `Triangle` image resizing filter. This is the
    /// fastest algorithm that produces acceptable results.
    ///
    /// To override this and use another image resizing filter, see
    /// `generate_mip_level_with_filter`.
    ///
    /// Currently only supports 2D textures in a few pixel formats. Will panic
    /// if used on an unsupported texture.
    pub fn generate_mipmaps(&mut self, max_mip_level: Option<usize>, min_size: Option<u32>) {
        self.generate_mipmaps_with_filter(max_mip_level, min_size, ImageResizeFilter::Triangle)
    }

    /// Variant of `generate_mipmaps` to allow specifying the filtering algorithm
    ///
    /// See that method's documentation for details
    pub fn generate_mipmaps_with_filter(
        &mut self,
        max_mip_level: Option<usize>,
        min_size: Option<u32>,
        filter: ImageResizeFilter,
    ) {
        if self.dimension != TextureDimension::D2 {
            panic!("Generating mipmaps is only supported for 2D textures.");
        }

        let mut mip_level = self.mipmaps.len() + 1;
        let mut mip_size = self.mip_size(mip_level);

        // PERF: this is inefficient: `texture_to_image` does a `.clone()` of the source data
        let base_image = super::image_texture_conversion::texture_to_image(self).unwrap();

        while mip_size.volume() > 1 {
            if let Some(max_level) = max_mip_level {
                if mip_level > max_level {
                    break;
                }
            }

            if let Some(min_size) = min_size {
                if mip_size.width < min_size || mip_size.height < min_size {
                    break;
                }
            }

            let resized = base_image.resize_exact(mip_size.width, mip_size.height, filter);
            let (_, _, new_data) = super::image_texture_conversion::image_to_texture_data(resized);
            self.mipmaps.push(new_data);

            mip_level += 1;
            mip_size = self.mip_size(mip_level);
        }
    }

    /// Convert a texture from a format to another
    /// Only a few formats are supported as input and output:
    /// - `TextureFormat::R8Unorm`
    /// - `TextureFormat::Rg8Unorm`
    /// - `TextureFormat::Rgba8UnormSrgb`
    /// - `TextureFormat::Bgra8UnormSrgb`
    ///
    /// (TODO) this method currently discards all mipmap levels except the base image
    pub fn convert(&self, new_format: TextureFormat) -> Option<Self> {
        super::image_texture_conversion::texture_to_image(self)
            .and_then(|img| match new_format {
                TextureFormat::R8Unorm => Some(image::DynamicImage::ImageLuma8(img.into_luma8())),
                TextureFormat::Rg8Unorm => {
                    Some(image::DynamicImage::ImageLumaA8(img.into_luma_alpha8()))
                }
                TextureFormat::Rgba8UnormSrgb => {
                    Some(image::DynamicImage::ImageRgba8(img.into_rgba8()))
                }
                TextureFormat::Bgra8UnormSrgb => {
                    Some(image::DynamicImage::ImageBgra8(img.into_bgra8()))
                }
                _ => None,
            })
            .map(super::image_texture_conversion::image_to_texture)
    }

    pub fn texture_resource_system(
        render_resource_context: Res<Box<dyn RenderResourceContext>>,
        textures: Res<Assets<Texture>>,
        mut texture_events: EventReader<AssetEvent<Texture>>,
    ) {
        let render_resource_context = &**render_resource_context;
        let mut changed_textures = HashSet::default();
        for event in texture_events.iter() {
            match event {
                AssetEvent::Created { handle } => {
                    changed_textures.insert(handle);
                }
                AssetEvent::Modified { handle } => {
                    changed_textures.insert(handle);
                    Self::remove_current_texture_resources(render_resource_context, handle);
                }
                AssetEvent::Removed { handle } => {
                    Self::remove_current_texture_resources(render_resource_context, handle);
                    // if texture was modified and removed in the same update, ignore the
                    // modification events are ordered so future modification
                    // events are ok
                    changed_textures.remove(handle);
                }
            }
        }

        for texture_handle in changed_textures.iter() {
            if let Some(texture) = textures.get(*texture_handle) {
                let texture_descriptor: TextureDescriptor = texture.into();
                let texture_resource = render_resource_context.create_texture(texture_descriptor);

                let sampler_resource = render_resource_context.create_sampler(&texture.sampler);

                render_resource_context.set_asset_resource(
                    texture_handle,
                    RenderResourceId::Texture(texture_resource),
                    TEXTURE_ASSET_INDEX,
                );
                render_resource_context.set_asset_resource(
                    texture_handle,
                    RenderResourceId::Sampler(sampler_resource),
                    SAMPLER_ASSET_INDEX,
                );
            }
        }
    }

    fn remove_current_texture_resources(
        render_resource_context: &dyn RenderResourceContext,
        handle: &Handle<Texture>,
    ) {
        if let Some(RenderResourceId::Texture(resource)) =
            render_resource_context.get_asset_resource(handle, TEXTURE_ASSET_INDEX)
        {
            render_resource_context.remove_texture(resource);
            render_resource_context.remove_asset_resource(handle, TEXTURE_ASSET_INDEX);
        }
        if let Some(RenderResourceId::Sampler(resource)) =
            render_resource_context.get_asset_resource(handle, SAMPLER_ASSET_INDEX)
        {
            render_resource_context.remove_sampler(resource);
            render_resource_context.remove_asset_resource(handle, SAMPLER_ASSET_INDEX);
        }
    }

    /// Load a bytes buffer in a [`Texture`], according to type `image_type`, using the `image`
    /// crate`
    pub fn from_buffer(buffer: &[u8], image_type: ImageType) -> Result<Texture, TextureError> {
        let format = match image_type {
            ImageType::MimeType(mime_type) => match mime_type {
                "image/png" => Ok(image::ImageFormat::Png),
                "image/vnd-ms.dds" => Ok(image::ImageFormat::Dds),
                "image/x-targa" => Ok(image::ImageFormat::Tga),
                "image/x-tga" => Ok(image::ImageFormat::Tga),
                "image/jpeg" => Ok(image::ImageFormat::Jpeg),
                "image/bmp" => Ok(image::ImageFormat::Bmp),
                "image/x-bmp" => Ok(image::ImageFormat::Bmp),
                _ => Err(TextureError::InvalidImageMimeType(mime_type.to_string())),
            },
            ImageType::Extension(extension) => image::ImageFormat::from_extension(extension)
                .ok_or_else(|| TextureError::InvalidImageMimeType(extension.to_string())),
        }?;

        // Load the image in the expected format.
        // Some formats like PNG allow for R or RG textures too, so the texture
        // format needs to be determined. For RGB textures an alpha channel
        // needs to be added, so the image data needs to be converted in those
        // cases.

        let dyn_img = image::load_from_memory_with_format(buffer, format)?;
        Ok(image_to_texture(dyn_img))
    }
}

impl RenderResource for Option<Handle<Texture>> {
    fn resource_type(&self) -> Option<RenderResourceType> {
        self.as_ref().map(|_texture| RenderResourceType::Texture)
    }

    fn write_buffer_bytes(&self, _buffer: &mut [u8]) {}

    fn buffer_byte_len(&self) -> Option<usize> {
        None
    }

    fn texture(&self) -> Option<&Handle<Texture>> {
        self.as_ref()
    }
}

impl RenderResource for Handle<Texture> {
    fn resource_type(&self) -> Option<RenderResourceType> {
        Some(RenderResourceType::Texture)
    }

    fn write_buffer_bytes(&self, _buffer: &mut [u8]) {}

    fn buffer_byte_len(&self) -> Option<usize> {
        None
    }

    fn texture(&self) -> Option<&Handle<Texture>> {
        Some(self)
    }
}

/// An error that occurs when loading a texture
#[derive(Error, Debug)]
pub enum TextureError {
    #[error("invalid image mime type")]
    InvalidImageMimeType(String),
    #[error("invalid image extension")]
    InvalidImageExtension(String),
    #[error("failed to load an image: {0}")]
    ImageError(#[from] image::ImageError),
}

/// Type of a raw image buffer
pub enum ImageType<'a> {
    /// Mime type of an image, for example `"image/png"`
    MimeType(&'a str),
    /// Extension of an image file, for example `"png"`
    Extension(&'a str),
}
