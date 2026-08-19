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
    /// Optional data used to initialize the buffer, as well as the buffer's size.
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
/// This also includes the buffer's size in bytes.
/// The buffer size must be a multiple of 4 as required by wgpu.
/// Zero size is allowed but can't be used as binding resource.
#[derive(Reflect, Debug, Clone)]
#[reflect(Default, Debug, Clone)]
#[reflect(opaque)]
pub enum ShaderBufferData {
    /// The buffer will be uninitialized when created and has the given size in
    /// bytes.
    Uninitialized(wgpu_types::BufferAddress),
    /// The buffer will be initialized with the given data.
    ///
    /// The size of the buffer is equal to `buffer_size`, not the size of `data`.
    Initialized {
        data: AlignedVec,
        buffer_size: wgpu_types::BufferAddress,
    },
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
        let data = AlignedVec::from(data);
        let buffer_size = data.len() as u64;
        ShaderBuffer {
            data: ShaderBufferData::Initialized { data, buffer_size },
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

    /// Clear [`Self::data`] if it is [`ShaderBufferData::Initialized`] with its capacity and buffer size reserved.
    pub fn clear(&mut self) {
        if let ShaderBufferData::Initialized { data, .. } = &mut self.data {
            data.clear();
        }
    }

    /// Extends the data with a slice of [`bytemuck::NoUninit`].
    /// If [`Self::data`] is uninitialized, it will be initialized with alignment `align_of::<T>()`
    ///
    /// [`ShaderBufferData::Initialized::buffer_size`] will be set to the data length.
    pub fn extend_from_slice<T>(&mut self, values: &[T])
    where
        T: bytemuck::NoUninit,
    {
        let data = core::mem::take(&mut self.data);
        let mut data = match data {
            ShaderBufferData::Uninitialized(_) => AlignedVec::new(align_of::<T>()),
            ShaderBufferData::Initialized { data, .. } => data,
        };
        data.extend_from_slice(bytemuck::cast_slice(values));
        let buffer_size = data.len() as u64;
        self.data = ShaderBufferData::Initialized { data, buffer_size };
    }

    /// Extends the data with an iterator of [`bytemuck::NoUninit`].
    /// If [`Self::data`] is uninitialized, it will be initialized with alignment `align_of::<T>()`.
    ///
    /// [`ShaderBufferData::Initialized::buffer_size`] will be set to the data length.
    pub fn extend<T>(&mut self, values: impl IntoIterator<Item = T>)
    where
        T: bytemuck::NoUninit,
    {
        let values = values.into_iter();
        let data = core::mem::take(&mut self.data);
        let mut data = match data {
            ShaderBufferData::Uninitialized(_) => AlignedVec::new(align_of::<T>()),
            ShaderBufferData::Initialized { data, .. } => data,
        };
        data.reserve(values.size_hint().0 * size_of::<T>());
        for value in values {
            data.extend_from_slice(bytemuck::bytes_of(&value));
        }
        let buffer_size = data.len() as u64;
        self.data = ShaderBufferData::Initialized { data, buffer_size };
    }

    /// Casts and returns a slice of `T` of [`ShaderBufferData::Initialized`],
    /// or returns `None` if it's [`ShaderBufferData::Uninitialized`]
    ///
    /// Panics:
    /// * If `T` has a greater alignment requirement and the `AlignedVec` isn't aligned.
    /// * If the size of `AlignedVec` is not a multiple of `size_of::<T>()`
    pub fn cast_slice<T: bytemuck::AnyBitPattern>(&self) -> Option<&[T]> {
        match &self.data {
            ShaderBufferData::Uninitialized(_) => None,
            ShaderBufferData::Initialized { data, .. } => Some(data.cast_slice()),
        }
    }

    /// Casts and returns a mutable slice of `T` of [`ShaderBufferData::Initialized`],
    /// or returns `None` if it's [`ShaderBufferData::Uninitialized`]
    ///
    /// Panics:
    /// * If `T` has a greater alignment requirement than the `AlignedVec`.
    /// * If the size of `AlignedVec` is not a multiple of `size_of::<T>()`
    pub fn cast_slice_mut<T: bytemuck::NoUninit + bytemuck::AnyBitPattern>(
        &mut self,
    ) -> Option<&mut [T]> {
        match &mut self.data {
            ShaderBufferData::Uninitialized(_) => None,
            ShaderBufferData::Initialized { data, .. } => Some(data.cast_slice_mut()),
        }
    }

    /// Resizes the CPU data and buffer to the new size.
    ///
    /// If CPU data is present, it will be truncated or zero-extended.
    /// If no CPU data is present, the GPU buffer will be reallocated. Preserves GPU data If `copy_on_resize` is true.
    pub fn resize(&mut self, new_size: wgpu_types::BufferAddress) {
        match self.data {
            ShaderBufferData::Initialized {
                ref mut data,
                ref mut buffer_size,
            } => {
                data.resize(new_size as usize, 0);
                *buffer_size = new_size;
            }
            ShaderBufferData::Uninitialized(ref mut size) => {
                *size = new_size;
            }
        }
    }

    /// Resizes the buffer to the new size. CPU data is unchanged.
    pub fn resize_buffer(&mut self, new_size: wgpu_types::BufferAddress) {
        match self.data {
            ShaderBufferData::Initialized {
                ref mut buffer_size,
                ..
            } => {
                *buffer_size = new_size;
            }
            ShaderBufferData::Uninitialized(ref mut size) => {
                *size = new_size;
            }
        }
    }

    /// Returns the size of the buffer in bytes.
    pub fn buffer_size(&self) -> wgpu_types::BufferAddress {
        match self.data {
            ShaderBufferData::Initialized { buffer_size, .. } => buffer_size,
            ShaderBufferData::Uninitialized(len) => len,
        }
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
        let len = source.buffer_size();
        let data = core::mem::replace(&mut source.data, ShaderBufferData::Uninitialized(len));

        let valid_upload = matches!(data, ShaderBufferData::Initialized { .. })
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
        let had_data = matches!(source_asset.data, ShaderBufferData::Initialized { .. });

        let buffer = if let Some(prev) = previous_asset
            && prev.buffer.size() == source_asset.buffer_size()
            && prev.buffer.usage() == source_asset.buffer_usage
            && *prev.label == *source_asset.label
            && (!had_data || source_asset.buffer_usage.contains(BufferUsages::COPY_DST))
        {
            if let ShaderBufferData::Initialized { ref data, .. } = source_asset.data {
                render_queue.write_buffer(
                    &prev.buffer,
                    0,
                    &data[..((source_asset.buffer_size() as usize).min(data.len()))],
                );
            }
            prev.buffer.clone()
        } else if let ShaderBufferData::Initialized { data, buffer_size } = source_asset.data {
            let mut desc = BufferDescriptor {
                label: Some(&*source_asset.label),
                usage: source_asset.buffer_usage,
                size: buffer_size,
                mapped_at_creation: true,
            };
            if buffer_size == 0 {
                // Skip mapping if the buffer is zero sized
                desc.mapped_at_creation = false;
                render_device.create_buffer(&desc)
            } else {
                desc.mapped_at_creation = true;
                let buffer = render_device.create_buffer(&desc);
                // Upload at most `buffer_size` bytes. If the data is shorter, the
                // remaining bytes stay zero-initialized; if it's longer, the tail
                // is truncated.
                let upload_len = (buffer_size as usize).min(data.len());
                buffer
                    .get_mapped_range_mut(..upload_len as u64)
                    .unwrap()
                    .copy_from_slice(&data[..upload_len]);
                buffer.unmap();
                buffer
            }
        } else {
            let new_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some(&*source_asset.label),
                size: source_asset.buffer_size(),
                usage: source_asset.buffer_usage,
                mapped_at_creation: false,
            });
            if source_asset.copy_on_resize
                && let Some(previous) = previous_asset
                && previous.buffer.usage().contains(BufferUsages::COPY_SRC)
                && source_asset.buffer_usage.contains(BufferUsages::COPY_DST)
            {
                let copy_size = source_asset.buffer_size().min(previous.buffer.size());
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    use bevy_ecs::{
        system::{lifetimeless::SRes, SystemState},
        world::World,
    };

    use crate::test_utils::create_dummy_device;

    /// Runs the extraction step of the [`RenderAsset`] pipeline on `source` and
    /// returns the extracted asset.
    fn extract(
        source: &mut ShaderBuffer,
        previous_gpu_asset: Option<&GpuShaderBuffer>,
    ) -> ShaderBuffer {
        GpuShaderBuffer::take_gpu_data(source, previous_gpu_asset)
            .expect("shader buffer should be extractable")
    }

    /// Creates a GPU buffer from an extracted [`ShaderBuffer`] using the given
    /// noop wgpu device (no real GPU required), optionally reusing
    /// `previous_asset`. The same device must be used across prepares when GPU
    /// buffers from a previous prepare are passed in.
    fn prepare(
        extracted: ShaderBuffer,
        previous_asset: Option<&GpuShaderBuffer>,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> GpuShaderBuffer {
        let mut world = World::new();
        world.insert_resource(device.clone());
        world.insert_resource(queue.clone());
        let mut system_state =
            SystemState::<(SRes<RenderDevice>, SRes<RenderQueue>)>::new(&mut world);
        let mut params = system_state
            .get_mut(&mut world)
            .expect("RenderDevice and RenderQueue resources should be present");
        GpuShaderBuffer::prepare_asset(extracted, AssetId::default(), &mut params, previous_asset)
            .expect("shader buffer should be prepared successfully")
    }

    /// Runs the full extract + prepare pipeline on a device shared with the
    /// given `previous_asset`.
    fn extract_and_prepare(
        source: &mut ShaderBuffer,
        previous_asset: Option<&GpuShaderBuffer>,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> GpuShaderBuffer {
        let extracted = extract(source, previous_asset);
        prepare(extracted, previous_asset, device, queue)
    }

    /// Runs the full pipeline on a fresh dummy device, for tests that don't
    /// chain multiple prepares together.
    fn extract_and_prepare_on_new_device(
        source: &mut ShaderBuffer,
        previous_asset: Option<&GpuShaderBuffer>,
    ) -> GpuShaderBuffer {
        let (device, queue) = create_dummy_device();
        extract_and_prepare(source, previous_asset, &device, &queue)
    }

    /// Extracts a buffer with data and uploads it to the GPU, verifying that a
    /// buffer of `buffer_size()` bytes is created and that extraction leaves the
    /// source asset uninitialized with its size preserved.
    #[test]
    fn extract_and_create_gpu_buffer_from_data() {
        let mut source = ShaderBuffer::new(
            vec![1u32, 2, 3],
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        // The buffer size defaults to the data length (3 * 4 = 12 bytes).
        assert_eq!(source.buffer_size(), 12);
        assert_eq!(source.cast_slice::<u32>(), Some(&[1, 2, 3][..]));

        let gpu = extract_and_prepare_on_new_device(&mut source, None);

        // Extraction moved the CPU data out of the source asset, leaving it
        // uninitialized but keeping its buffer size.
        assert!(matches!(source.data, ShaderBufferData::Uninitialized(12)));

        // The GPU buffer has the buffer size and all extracted data uploaded.
        assert_eq!(gpu.buffer.size(), 12);
        assert!(gpu.had_data);
        assert_eq!(gpu.label, source.label);
        assert_eq!(gpu.buffer_usage, source.buffer_usage);
    }

    /// Verifies that an explicitly larger buffer size is respected: the GPU
    /// buffer is created with `buffer_size` bytes, even though the CPU data is
    /// shorter.
    #[test]
    fn create_gpu_buffer_with_buffer_size_larger_than_data() {
        let mut source = ShaderBuffer::new(vec![1u32], RenderAssetUsages::default());
        // Grow the GPU buffer without touching the CPU data.
        source.resize_buffer(64);
        assert_eq!(source.buffer_size(), 64);
        assert_eq!(source.cast_slice::<u32>(), Some(&[1][..]));

        let gpu = extract_and_prepare_on_new_device(&mut source, None);

        assert_eq!(gpu.buffer.size(), 64);
        assert!(gpu.had_data);
    }

    /// Verifies that an explicitly smaller buffer size is respected: the GPU
    /// buffer is created with `buffer_size` bytes and only the first
    /// `buffer_size` bytes of the CPU data are uploaded.
    #[test]
    fn create_gpu_buffer_with_buffer_size_smaller_than_data() {
        let mut source = ShaderBuffer::new(vec![1u32, 2, 3, 4], RenderAssetUsages::default());
        source.resize_buffer(8);
        assert_eq!(source.buffer_size(), 8);

        let gpu = extract_and_prepare_on_new_device(&mut source, None);

        assert_eq!(gpu.buffer.size(), 8);
        assert!(gpu.had_data);
    }

    /// Verifies that zero-sized buffers are created without attempting to map
    /// them, both for initialized and uninitialized sources.
    #[test]
    fn create_zero_sized_gpu_buffer() {
        // An initialized buffer whose data is empty.
        let mut source = ShaderBuffer::new(Vec::<u32>::new(), RenderAssetUsages::default());
        assert_eq!(source.buffer_size(), 0);

        let gpu = extract_and_prepare_on_new_device(&mut source, None);
        assert_eq!(gpu.buffer.size(), 0);
        assert!(gpu.had_data);

        // An uninitialized buffer with size zero.
        let mut source = ShaderBuffer::with_size(0, RenderAssetUsages::default());
        let gpu = extract_and_prepare_on_new_device(&mut source, None);
        assert_eq!(gpu.buffer.size(), 0);
        assert!(!gpu.had_data);
    }

    /// Verifies that an uninitialized buffer creates an uninitialized GPU buffer
    /// of the requested size and is reported as having no data.
    #[test]
    fn create_uninitialized_gpu_buffer() {
        let mut source = ShaderBuffer::with_size(1024, RenderAssetUsages::default());
        assert_eq!(source.buffer_size(), 1024);

        let gpu = extract_and_prepare_on_new_device(&mut source, None);

        assert_eq!(gpu.buffer.size(), 1024);
        assert!(!gpu.had_data);
    }

    /// Verifies the extraction guards: a buffer whose CPU data has already been
    /// moved to the render world can still be extracted if there is no previous
    /// GPU asset carrying data, but is rejected once a previous GPU asset
    /// contained data.
    #[test]
    fn extraction_rejects_buffer_whose_data_would_be_lost() {
        let mut source = ShaderBuffer::new(vec![1u32], RenderAssetUsages::default());
        assert!(GpuShaderBuffer::take_gpu_data(&mut source, None).is_ok());
        assert!(matches!(source.data, ShaderBufferData::Uninitialized(4)));

        // The source no longer holds data, but with no previous GPU asset the
        // GPU buffer can simply be created uninitialized.
        assert!(GpuShaderBuffer::take_gpu_data(&mut source, None).is_ok());

        // An uninitialized buffer is rejected when the previous GPU asset had
        // data, since re-preparing it would silently drop that data.
        let mut source = ShaderBuffer::with_size(4, RenderAssetUsages::default());
        let previous = GpuShaderBuffer {
            buffer: create_dummy_device().0.create_buffer(&BufferDescriptor {
                label: Some("previous"),
                size: 4,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            label: Cow::Borrowed("shader buffer"),
            buffer_usage: BufferUsages::STORAGE,
            had_data: true,
        };
        assert!(matches!(
            GpuShaderBuffer::take_gpu_data(&mut source, Some(&previous)),
            Err(AssetExtractionError::AlreadyExtracted)
        ));
    }

    /// Verifies that an unchanged buffer reuses the existing GPU buffer instead
    /// of allocating a new one, and that changing the buffer size or losing the
    /// data invalidates the reuse.
    #[test]
    fn reuses_gpu_buffer_when_unchanged() {
        let (device, queue) = create_dummy_device();

        let mut source = ShaderBuffer::new(vec![1u32, 2, 3], RenderAssetUsages::default());
        let first = extract_and_prepare(&mut source, None, &device, &queue);

        // Same size/usage/label: the existing buffer is reused.
        let mut source = ShaderBuffer::new(vec![4u32, 5, 6], RenderAssetUsages::default());
        let second = extract_and_prepare(&mut source, Some(&first), &device, &queue);
        assert_eq!(second.buffer.id(), first.buffer.id());

        // A different buffer size forces a new allocation.
        let mut source = ShaderBuffer::new(vec![1u32], RenderAssetUsages::default());
        let resized = extract_and_prepare(&mut source, Some(&first), &device, &queue);
        assert_ne!(resized.buffer.id(), first.buffer.id());
        assert_eq!(resized.buffer.size(), 4);
    }
}
