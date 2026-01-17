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
use encase::{internal::WriteInto, ShaderType};
use wgpu::util::BufferInitDescriptor;

/// Adds [`ShaderStorageBuffer`] as an asset that is extracted and uploaded to the GPU.
#[derive(Default)]
pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<GpuShaderStorageBuffer>::default())
            .init_asset::<ShaderStorageBuffer>()
            .register_asset_reflect::<ShaderStorageBuffer>();
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect(opaque)]
#[reflect(Default, Debug, Clone)]
pub struct ShaderStorageBuffer {
    /// Optional data used to initialize the buffer.
    pub data: Option<Vec<u8>>,
    /// The buffer description used to create the buffer.
    pub buffer_description: wgpu::BufferDescriptor<'static>,
    /// The asset usage of the storage buffer.
    pub asset_usage: RenderAssetUsages,
    /// Whether this buffer should be copied on the GPU when resized.
    pub copy_on_resize: bool,
}

impl Default for ShaderStorageBuffer {
    fn default() -> Self {
        Self {
            data: None,
            buffer_description: wgpu::BufferDescriptor {
                label: None,
                size: 0,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            },
            asset_usage: RenderAssetUsages::default(),
            copy_on_resize: false,
        }
    }
}

impl ShaderStorageBuffer {
    /// Creates a new storage buffer with the given data and asset usage.
    pub fn new(data: &[u8], asset_usage: RenderAssetUsages) -> Self {
        let mut storage = ShaderStorageBuffer {
            data: Some(data.to_vec()),
            ..default()
        };
        storage.asset_usage = asset_usage;
        storage
    }

    /// Creates a new storage buffer with the given size and asset usage.
    pub fn with_size(size: usize, asset_usage: RenderAssetUsages) -> Self {
        let mut storage = ShaderStorageBuffer {
            data: None,
            ..default()
        };
        storage.buffer_description.size = size as u64;
        storage.buffer_description.mapped_at_creation = false;
        storage.asset_usage = asset_usage;
        storage
    }

    /// Sets the data of the storage buffer to the given [`ShaderType`].
    pub fn set_data<T>(&mut self, value: T)
    where
        T: ShaderType + WriteInto,
    {
        let size = value.size().get() as usize;
        let mut wrapper = encase::StorageBuffer::<Vec<u8>>::new(Vec::with_capacity(size));
        wrapper.write(&value).unwrap();
        self.data = Some(wrapper.into_inner());
    }

    /// Resizes the buffer to the new size.
    ///
    /// If CPU data is present, it will be truncated or zero-extended.
    /// Does not preserve GPU data when the descriptor changes.
    pub fn resize(&mut self, size: u64) {
        self.buffer_description.size = size;
        if let Some(ref mut data) = self.data {
            data.resize(size as usize, 0);
        }
    }

    /// Resizes the buffer to the new size, preserving existing data.
    ///
    /// If CPU data is present, it will be truncated or zero-extended.
    /// If no CPU data is present, sets `copy_on_resize` to preserve GPU data.
    pub fn resize_in_place(&mut self, size: u64) {
        self.buffer_description.size = size;
        if let Some(ref mut data) = self.data {
            data.resize(size as usize, 0);
        } else {
            self.copy_on_resize = true;
        }
    }
}

impl<T> From<T> for ShaderStorageBuffer
where
    T: ShaderType + WriteInto,
{
    fn from(value: T) -> Self {
        let size = value.size().get() as usize;
        let mut wrapper = encase::StorageBuffer::<Vec<u8>>::new(Vec::with_capacity(size));
        wrapper.write(&value).unwrap();
        Self::new(wrapper.as_ref(), RenderAssetUsages::default())
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
pub struct GpuShaderStorageBuffer {
    pub buffer: Buffer,
    pub buffer_descriptor: wgpu::BufferDescriptor<'static>,
    pub had_data: bool,
}

impl RenderAsset for GpuShaderStorageBuffer {
    type SourceAsset = ShaderStorageBuffer;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn asset_usage(source_asset: &Self::SourceAsset) -> RenderAssetUsages {
        source_asset.asset_usage
    }

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        previous_gpu_asset: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        let data = source.data.take();

        let valid_upload = data.is_some() || previous_gpu_asset.is_none_or(|prev| !prev.had_data);

        valid_upload
            .then(|| Self::SourceAsset {
                data,
                ..source.clone()
            })
            .ok_or(AssetExtractionError::AlreadyExtracted)
    }

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_device, render_queue): &mut SystemParamItem<Self::Param>,
        previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let had_data = source_asset.data.is_some();

        let buffer = if let Some(prev) = previous_asset
            && prev.buffer_descriptor == source_asset.buffer_description
        {
            if let Some(ref data) = source_asset.data {
                render_queue.write_buffer(&prev.buffer, 0, data);
            }
            prev.buffer.clone()
        } else if let Some(ref data) = source_asset.data {
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: source_asset.buffer_description.label,
                contents: data,
                usage: source_asset.buffer_description.usage,
            })
        } else {
            let new_buffer = render_device.create_buffer(&source_asset.buffer_description);
            if source_asset.copy_on_resize
                && let Some(previous) = previous_asset
            {
                let copy_size = source_asset
                    .buffer_description
                    .size
                    .min(previous.buffer_descriptor.size);
                let mut encoder =
                    render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("copy_buffer_on_resize"),
                    });
                encoder.copy_buffer_to_buffer(&previous.buffer, 0, &new_buffer, 0, copy_size);
                render_queue.submit([encoder.finish()]);
            }
            new_buffer
        };

        Ok(GpuShaderStorageBuffer {
            buffer,
            buffer_descriptor: source_asset.buffer_description,
            had_data,
        })
    }
}
