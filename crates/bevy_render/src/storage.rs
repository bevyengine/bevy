//! [`ShaderBuffer`], an asset that encapsulates arbitrary data that will be
//! extracted and uploaded to the GPU for use in shaders.

use alloc::borrow::Cow;
use bevy_platform::collections::AlignedVec;

use crate::{
    render_asset::{AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin},
    render_resource::{Buffer, BufferUsages},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetId, RenderAssetUsages};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_utils::default;
use wgpu::util::BufferInitDescriptor;
use wgpu_types::BufferDescriptor;

/// Adds a [`ShaderBuffer`] as an asset that is extracted and uploaded to the GPU.
#[derive(Default)]
pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<GpuShaderBuffer>::default())
            .init_asset::<ShaderBuffer>()
            .register_asset_reflect::<ShaderBuffer>();
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
///
/// This buffer primarily exists in order to be embedded into a material that
/// implements the [`bevy_render_macros::AsBindGroup`] trait. Compared to
/// embedding a raw [`Buffer`], [`ShaderBuffer`] has the advantage that the
/// buffer can be resized without regenerating the materials that embed it.
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect(opaque)]
#[reflect(Default, Debug, Clone)]
pub struct ShaderBuffer {
    /// Optional data used to initialize the buffer, as well as the buffer's
    /// minimum size.
    pub data: ShaderBufferData,
    /// A label that can be used to identify this buffer in a debugger.
    pub label: Cow<'static, str>,
    /// How this buffer can legally be used.
    pub buffer_usage: BufferUsages,
    /// The asset usage of the storage buffer.
    pub asset_usage: RenderAssetUsages,
    /// Whether this buffer should be copied on the GPU when resized.
    /// The buffer should have `BufferUsages::COPY_SRC | BufferUsages::COPY_DST` usages to be copyable.
    pub copy_on_resize: bool,
}

/// Optional data used to initialize a [`ShaderBuffer`].
///
/// This also includes the buffer's minimum size in bytes. Note that the actual
/// length of the buffer may be larger than this in order to amortize
/// reallocations.
#[derive(Reflect, Debug, Clone)]
#[reflect(Default, Debug, Clone)]
#[reflect(opaque)]
pub enum ShaderBufferData {
    /// The buffer will be uninitialized when created and has the given size in
    /// bytes.
    Uninitialized(wgpu_types::BufferAddress),
    /// The buffer will be initialized with the given data.
    ///
    /// The size of the buffer is equal to the size of the data.
    Initialized(AlignedVec),
}

impl Default for ShaderBuffer {
    fn default() -> Self {
        Self {
            data: ShaderBufferData::Uninitialized(0),
            label: Cow::Borrowed("shader buffer"),
            buffer_usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            asset_usage: RenderAssetUsages::default(),
            copy_on_resize: false,
        }
    }
}

impl Default for ShaderBufferData {
    fn default() -> Self {
        ShaderBufferData::Uninitialized(0)
    }
}

impl ShaderBuffer {
    /// Creates a new initialized storage buffer with the given data and asset usage, with alignment `align_of::<T>()`.
    pub fn new<T: bytemuck::NoUninit>(data: Vec<T>, asset_usage: RenderAssetUsages) -> Self {
        ShaderBuffer {
            data: ShaderBufferData::Initialized(AlignedVec::from(data)),
            asset_usage,
            ..default()
        }
    }

    /// Creates a new uninitialized storage buffer with the given size and asset usage.
    pub fn with_size(size: u64, asset_usage: RenderAssetUsages) -> Self {
        ShaderBuffer {
            data: ShaderBufferData::Uninitialized(size),
            asset_usage,
            ..default()
        }
    }

    /// Clear [`Self::data`] with its capacity reserved if it is [`ShaderBufferData::Initialized`].
    pub fn clear(&mut self) {
        if let ShaderBufferData::Initialized(data) = &mut self.data {
            data.clear();
        }
    }

    /// Extends the data with a slice of [`bytemuck::NoUninit`].
    /// If [`Self::data`] is uninitialized, it will be initialized with alignment `align_of::<T>()`
    pub fn extend_from_slice<T>(&mut self, values: &[T])
    where
        T: bytemuck::NoUninit,
    {
        let data = core::mem::take(&mut self.data);
        let mut data = match data {
            ShaderBufferData::Uninitialized(_) => AlignedVec::new(align_of::<T>()),
            ShaderBufferData::Initialized(aligned_vec) => aligned_vec,
        };
        data.extend_from_slice(bytemuck::cast_slice(values));
        self.data = ShaderBufferData::Initialized(data);
    }

    /// Extends the data with an iterator of [`bytemuck::NoUninit`].
    /// If [`Self::data`] is uninitialized, it will be initialized with alignment `align_of::<T>()`
    pub fn extend<T>(&mut self, values: impl IntoIterator<Item = T>)
    where
        T: bytemuck::NoUninit,
    {
        let values = values.into_iter();
        let data = core::mem::take(&mut self.data);
        let mut data = match data {
            ShaderBufferData::Uninitialized(_) => AlignedVec::new(align_of::<T>()),
            ShaderBufferData::Initialized(aligned_vec) => aligned_vec,
        };
        data.reserve(values.size_hint().0 * size_of::<T>());
        for value in values {
            data.extend_from_slice(bytemuck::bytes_of(&value));
        }
        self.data = ShaderBufferData::Initialized(data);
    }

    /// Returns a slice of `T` of [`ShaderBufferData::Initialized`], otherwise returns `None`
    ///
    /// Panics:
    /// * If `T` has a greater alignment requirement and the `AlignedVec` isn't aligned.
    /// * If the size of `AlignedVec` is not a multiple of `size_of::<T>()`
    pub fn as_slice<T: bytemuck::AnyBitPattern>(&self) -> Option<&[T]> {
        match &self.data {
            ShaderBufferData::Uninitialized(_) => None,
            ShaderBufferData::Initialized(aligned_vec) => Some(aligned_vec.cast_slice()),
        }
    }

    /// Returns a mutable slice of `T` of [`ShaderBufferData::Initialized`], otherwise returns `None`
    ///
    /// Panics:
    /// * If `T` has a greater alignment requirement than the `AlignedVec`.
    /// * If the size of `AlignedVec` is not a multiple of `size_of::<T>()`
    pub fn as_mut_slice<T: bytemuck::NoUninit + bytemuck::AnyBitPattern>(
        &mut self,
    ) -> Option<&mut [T]> {
        match &mut self.data {
            ShaderBufferData::Uninitialized(_) => None,
            ShaderBufferData::Initialized(aligned_vec) => Some(aligned_vec.cast_slice_mut()),
        }
    }

    /// Resizes the buffer to the new size.
    ///
    /// If CPU data is present, it will be truncated or zero-extended.
    /// If no CPU data is present, the GPU buffer will be reallocated. Preserves GPU data If `copy_on_resize` is true.
    pub fn resize(&mut self, new_size: wgpu_types::BufferAddress) {
        match self.data {
            ShaderBufferData::Initialized(ref mut data) => {
                data.resize(new_size as usize, 0);
            }
            ShaderBufferData::Uninitialized(ref mut size) => {
                *size = new_size;
            }
        }
    }

    /// Returns the size of the buffer in bytes.
    pub fn len(&self) -> wgpu_types::BufferAddress {
        match self.data {
            ShaderBufferData::Initialized(ref data) => data.len() as wgpu_types::BufferAddress,
            ShaderBufferData::Uninitialized(len) => len,
        }
    }

    /// Returns true if the buffer is empty or false if the buffer contains some
    /// data.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: bytemuck::NoUninit> From<Vec<T>> for ShaderBuffer {
    /// Creates a new initialized storage buffer with the given data, with alignment `align_of::<T>()`.
    fn from(value: Vec<T>) -> Self {
        Self::new(value, Default::default())
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
pub struct GpuShaderBuffer {
    /// The raw GPU buffer.
    pub buffer: Buffer,
    /// A debugging label to identify the buffer.
    pub label: Cow<'static, str>,
    /// The allowable render usages of the buffer.
    pub buffer_usage: BufferUsages,
    /// Whether the buffer contains data that must be preserved.
    pub had_data: bool,
}

impl RenderAsset for GpuShaderBuffer {
    type SourceAsset = ShaderBuffer;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn asset_usage(source_asset: &Self::SourceAsset) -> RenderAssetUsages {
        source_asset.asset_usage
    }

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        previous_gpu_asset: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        let len = source.len();
        let data = core::mem::replace(&mut source.data, ShaderBufferData::Uninitialized(len));

        let valid_upload = matches!(data, ShaderBufferData::Initialized(_))
            || previous_gpu_asset.is_none_or(|prev| !prev.had_data);

        valid_upload
            .then(|| Self::SourceAsset {
                data,
                ..source.clone()
            })
            .ok_or(AssetExtractionError::AlreadyExtracted)
    }

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        &mut (ref render_device, ref render_queue): &mut SystemParamItem<Self::Param>,
        previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let had_data = matches!(source_asset.data, ShaderBufferData::Initialized(_));

        let buffer = if let Some(prev) = previous_asset
            && prev.buffer.size() == source_asset.len()
            && prev.buffer.usage() == source_asset.buffer_usage
            && *prev.label == *source_asset.label
            && (!had_data || source_asset.buffer_usage.contains(BufferUsages::COPY_DST))
        {
            if let ShaderBufferData::Initialized(ref data) = source_asset.data {
                render_queue.write_buffer(&prev.buffer, 0, data);
            }
            prev.buffer.clone()
        } else if let ShaderBufferData::Initialized(ref data) = source_asset.data {
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(&*source_asset.label),
                contents: data,
                usage: source_asset.buffer_usage,
            })
        } else {
            let new_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some(&*source_asset.label),
                size: source_asset.len(),
                usage: source_asset.buffer_usage,
                mapped_at_creation: false,
            });
            if source_asset.copy_on_resize
                && let Some(previous) = previous_asset
                && previous.buffer.usage().contains(BufferUsages::COPY_SRC)
                && source_asset.buffer_usage.contains(BufferUsages::COPY_DST)
            {
                let copy_size = source_asset.len().min(previous.buffer.size());
                let mut encoder =
                    render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("copy_buffer_on_resize"),
                    });
                encoder.copy_buffer_to_buffer(&previous.buffer, 0, &new_buffer, 0, copy_size);
                render_queue.submit([encoder.finish()]);
            }
            new_buffer
        };

        Ok(GpuShaderBuffer {
            buffer,
            label: source_asset.label,
            buffer_usage: source_asset.buffer_usage,
            had_data,
        })
    }
}
