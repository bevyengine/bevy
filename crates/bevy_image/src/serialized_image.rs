use crate::{Image, ImageSampler, ImageTextureDescriptor, ImageTextureViewDescriptor};
use bevy_asset::RenderAssetUsages;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use wgpu_types::{TextureDataOrder, TextureDescriptor};

/// A version of [`Image`] suitable for serializing for short-term transfer.
///
/// [`Image`] does not implement [`Serialize`] / [`Deserialize`] because it is made with the renderer in mind.
/// It is not a general-purpose image implementation, and its internals are subject to frequent change.
/// As such, storing an [`Image`] on disk is highly discouraged.
/// Use an existing image asset format such as `.png` instead!
///
/// But there are still some valid use cases for serializing an [`Image`], namely transferring images between processes.
/// To support this, you can create a [`SerializedImage`] from an [`Image`] with [`SerializedImage::from_image`],
/// and then deserialize it with [`SerializedImage::into_image`].
///
/// The caveats are:
/// - The image representation is not valid across different versions of Bevy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedImage {
    data: Option<Vec<u8>>,
    data_order: TextureDataOrder,
    texture_descriptor: ImageTextureDescriptor,
    sampler: ImageSampler,
    texture_view_descriptor: Option<ImageTextureViewDescriptor>,
}

impl SerializedImage {
    /// Creates a new [`SerializedImage`] from an [`Image`].
    pub fn from_image(image: Image) -> Self {
        Self {
            data: image.data,
            data_order: image.data_order,
            texture_descriptor: image.texture_descriptor,
            sampler: image.sampler,
            texture_view_descriptor: image.texture_view_descriptor,
        }
    }

    /// Create an [`Image`] from a [`SerializedImage`].
    pub fn into_image(self) -> Image {
        Image {
            data: self.data,
            data_order: self.data_order,
            texture_descriptor: TextureDescriptor {
                label: self.texture_descriptor.label,
                size: self.texture_descriptor.size,
                mip_level_count: self.texture_descriptor.mip_level_count,
                sample_count: self.texture_descriptor.sample_count,
                dimension: self.texture_descriptor.dimension,
                format: self.texture_descriptor.format,
                usage: self.texture_descriptor.usage,
                view_formats: self.texture_descriptor.view_formats,
            },
            sampler: self.sampler,
            texture_view_descriptor: self.texture_view_descriptor,
            asset_usage: RenderAssetUsages::default(),
            copy_on_resize: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

    use super::*;

    #[test]
    fn serialize_deserialize_image() {
        let image = Image::new(
            Extent3d {
                width: 3,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );

        let serialized_image = SerializedImage::from_image(image.clone());
        let serialized_string = serde_json::to_string(&serialized_image).unwrap();
        let serialized_image_from_string: SerializedImage =
            serde_json::from_str(&serialized_string).unwrap();
        let deserialized_image = serialized_image_from_string.into_image();
        assert_eq!(image, deserialized_image);
    }
}
