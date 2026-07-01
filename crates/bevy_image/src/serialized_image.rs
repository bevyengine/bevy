use crate::{Image, ImageSampler, ImageTextureDescriptor};
use alloc::borrow::Cow;
use bevy_asset::RenderAssetUsages;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use wgpu_types::{
    TextureAspect, TextureDataOrder, TextureDescriptor, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};

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
    data_order: SerializedTextureDataOrder,
    texture_descriptor: ImageTextureDescriptor,
    sampler: ImageSampler,
    texture_view_descriptor: Option<SerializedTextureViewDescriptor>,
}

// TODO: Serialize and Deserialize have been added to `TextureViewDescriptor` in wgpu. Remove this once wgpu is updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializedTextureViewDescriptor {
    label: Option<Cow<'static, str>>,
    format: Option<TextureFormat>,
    dimension: Option<TextureViewDimension>,
    usage: Option<TextureUsages>,
    aspect: TextureAspect,
    base_mip_level: u32,
    mip_level_count: Option<u32>,
    base_array_layer: u32,
    array_layer_count: Option<u32>,
}

impl SerializedTextureViewDescriptor {
    fn from_texture_view_descriptor(
        descriptor: TextureViewDescriptor<Option<impl Into<Cow<'static, str>>>>,
    ) -> Self {
        Self {
            label: descriptor.label.map(Into::into),
            format: descriptor.format,
            dimension: descriptor.dimension,
            usage: descriptor.usage,
            aspect: descriptor.aspect,
            base_mip_level: descriptor.base_mip_level,
            mip_level_count: descriptor.mip_level_count,
            base_array_layer: descriptor.base_array_layer,
            array_layer_count: descriptor.array_layer_count,
        }
    }

    fn into_texture_view_descriptor<L: From<Cow<'static, str>>>(
        self,
    ) -> TextureViewDescriptor<Option<L>> {
        TextureViewDescriptor {
            label: self.label.map(L::from),
            format: self.format,
            dimension: self.dimension,
            usage: self.usage,
            aspect: self.aspect,
            base_mip_level: self.base_mip_level,
            mip_level_count: self.mip_level_count,
            base_array_layer: self.base_array_layer,
            array_layer_count: self.array_layer_count,
        }
    }
}

// TODO: Serialize and Deserialize have been added to `TextureDataOrder` in wgpu. Remove this once wgpu is updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializedTextureDataOrder {
    LayerMajor,
    MipMajor,
}

impl SerializedTextureDataOrder {
    fn from_texture_data_order(order: TextureDataOrder) -> Self {
        match order {
            TextureDataOrder::LayerMajor => SerializedTextureDataOrder::LayerMajor,
            TextureDataOrder::MipMajor => SerializedTextureDataOrder::MipMajor,
        }
    }

    fn into_texture_data_order(self) -> TextureDataOrder {
        match self {
            SerializedTextureDataOrder::LayerMajor => TextureDataOrder::LayerMajor,
            SerializedTextureDataOrder::MipMajor => TextureDataOrder::MipMajor,
        }
    }
}

impl SerializedImage {
    /// Creates a new [`SerializedImage`] from an [`Image`].
    pub fn from_image(image: Image) -> Self {
        Self {
            data: image.data,
            data_order: SerializedTextureDataOrder::from_texture_data_order(image.data_order),
            texture_descriptor: image.texture_descriptor,
            sampler: image.sampler,
            texture_view_descriptor: image.texture_view_descriptor.map(|descriptor| {
                SerializedTextureViewDescriptor::from_texture_view_descriptor(descriptor)
            }),
        }
    }

    /// Create an [`Image`] from a [`SerializedImage`].
    pub fn into_image(self) -> Image {
        Image {
            data: self.data,
            data_order: self.data_order.into_texture_data_order(),
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
            texture_view_descriptor: self
                .texture_view_descriptor
                .map(SerializedTextureViewDescriptor::into_texture_view_descriptor),
            asset_usage: RenderAssetUsages::default(),
            copy_on_resize: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use wgpu_types::{Extent3d, TextureDimension};

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
