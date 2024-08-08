use crate::render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages};
use crate::render_resource::{Buffer, BufferUsages};
use crate::renderer::RenderDevice;
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp};
use bevy_ecs::system::lifetimeless::SRes;
use bevy_ecs::system::SystemParamItem;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_utils::default;
use wgpu::util::BufferInitDescriptor;

/// Adds [`Storage`] as an asset that is extracted and uploaded to the GPU.
#[derive(Default)]
pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<GpuStorageBuffer>::default())
            .register_type::<Storage>()
            .init_asset::<Storage>()
            .register_asset_reflect::<Storage>();
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect_value(Default)]
pub struct Storage {
    /// Optional data used to initialize the buffer.
    pub data: Option<Vec<u8>>,
    /// The buffer description used to create the buffer.
    pub buffer_description: wgpu::BufferDescriptor<'static>,
    /// The asset usage of the storage buffer.
    pub asset_usage: RenderAssetUsages,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            data: None,
            buffer_description: wgpu::BufferDescriptor {
                label: None,
                size: 0,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            },
            asset_usage: RenderAssetUsages::empty(),
        }
    }
}

impl Storage {
    /// Creates a new storage buffer with the given data and asset usage.
    pub fn new(data: &[u8], asset_usage: RenderAssetUsages) -> Self {
        let mut storage = Storage {
            data: Some(data.to_vec()),
            ..default()
        };
        storage.asset_usage = asset_usage;
        storage
    }

    /// Creates a new storage buffer with the given size and asset usage.
    pub fn with_size(size: usize, asset_usage: RenderAssetUsages) -> Self {
        let mut storage = Storage {
            data: None,
            ..default()
        };
        storage.buffer_description.size = size as u64;
        storage.buffer_description.mapped_at_creation = true;
        storage.asset_usage = asset_usage;
        storage
    }
}

/// A storage buffer that is prepared as a [`RenderAsset`] and uploaded to the GPU.
pub struct GpuStorageBuffer {
    pub buffer: Buffer,
}

impl RenderAsset for GpuStorageBuffer {
    type SourceAsset = Storage;
    type Param = SRes<RenderDevice>;

    fn asset_usage(source_asset: &Self::SourceAsset) -> RenderAssetUsages {
        source_asset.asset_usage
    }

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        match source_asset.data {
            Some(data) => {
                let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: source_asset.buffer_description.label,
                    contents: &data,
                    usage: source_asset.buffer_description.usage,
                });
                Ok(GpuStorageBuffer { buffer })
            }
            None => {
                let buffer = render_device.create_buffer(&source_asset.buffer_description);
                Ok(GpuStorageBuffer { buffer })
            }
        }
    }
}
