use crate::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages},
    render_resource::{Buffer, BufferUsages},
    renderer::RenderDevice,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetId};
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
}

impl Default for ShaderStorageBuffer {
    fn default() -> Self {
        Self {
            data: None,
            buffer_description: wgpu::BufferDescriptor {
                label: None,
                size: 0,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            },
            asset_usage: RenderAssetUsages::default(),
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
}

impl RenderAsset for GpuShaderStorageBuffer {
    type SourceAsset = ShaderStorageBuffer;
    type Param = SRes<RenderDevice>;

    fn asset_usage(source_asset: &Self::SourceAsset) -> RenderAssetUsages {
        source_asset.asset_usage
    }

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        render_device: &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        match source_asset.data {
            Some(data) => {
                let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: source_asset.buffer_description.label,
                    contents: &data,
                    usage: source_asset.buffer_description.usage,
                });
                Ok(GpuShaderStorageBuffer { buffer })
            }
            None => {
                let buffer = render_device.create_buffer(&source_asset.buffer_description);
                Ok(GpuShaderStorageBuffer { buffer })
            }
        }
    }
}
