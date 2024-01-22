//! Irradiance volumes.

use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    query::QueryItem,
    schedule::IntoSystemConfigs as _,
    system::{lifetimeless::Read, Res, ResMut, Resource, SystemParamItem},
    world::{FromWorld, World},
};
use bevy_render::{
    extract_instances::{ExtractInstance, ExtractInstancesPlugin, ExtractedInstances},
    render_asset::{
        PrepareAssetError, RenderAsset, RenderAssetPersistencePolicy, RenderAssetPlugin,
        RenderAssets,
    },
    render_resource::{GpuArrayBuffer, GpuArrayBufferIndex, Shader, ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bevy_transform::components::GlobalTransform;
use byteorder::{LittleEndian, ReadBytesExt as _};
use std::{
    io::{self, Cursor, Read as _},
    marker::PhantomData,
};
use thiserror::Error;

use bevy_asset::{
    io::Reader, load_internal_asset, Asset, AssetApp as _, AssetId, AssetLoader, AsyncReadExt,
    Handle, LoadContext,
};
use bevy_math::{uvec3, Mat4, UVec3};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_utils::{BoxedFuture, HashMap, HashSet};

pub static IRRADIANCE_VOXELS_EXTENSION: &str = "vxgi";
pub static IRRADIANCE_VOXELS_MAGIC_NUMBER: &[u8; 4] = b"VXGI";
pub const IRRADIANCE_VOXELS_VERSION: u32 = 0;

pub const IRRADIANCE_VOLUMES_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(160299515939076705258408299184317675488);

pub const MAX_IRRADIANCE_VOLUMES: usize = 256;

/// The component that defines an irradiance volume.
#[derive(Clone, Default, Reflect, Component, Debug, TypeUuid)]
#[uuid = "692f12fb-b566-4c28-bf63-e0bc5ee4df87"]
pub struct IrradianceVolume {
    pub voxels: Handle<IrradianceVoxels>,
    pub intensity: f32,
}

#[derive(Clone, Reflect, Asset)]
pub struct IrradianceVoxels {
    /// The size of the voxel grid, in voxels.
    pub resolution: UVec3,

    /// The voxel grid data, stored as 32-bit floating point RGBA.
    /// TODO(pcwalton): Switch to RGB9e5.
    pub data: Vec<u32>,
}

#[derive(Default)]
pub struct IrradianceVoxelsAssetLoader;

#[derive(Error, Debug)]
pub enum IrradianceVoxelsAssetLoadError {
    #[error("unknown magic number")]
    BadMagicNumber,
    #[error("unknown version")]
    BadVersion,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuIrradianceVolume {
    pub transform: Mat4,
    pub inverse_transform: Mat4,
    pub resolution: UVec3,
    pub start_offset: u32,
}

#[derive(ShaderType)]
pub struct GpuIrradianceVolumes {
    pub volumes: [GpuIrradianceVolume; MAX_IRRADIANCE_VOLUMES],
}

#[derive(Resource)]
pub struct RenderIrradianceVolumes {
    pub gpu_irradiance_volume_metadata: UniformBuffer<GpuIrradianceVolumes>,
    pub gpu_irradiance_volumes: GpuArrayBuffer<u32>,
    pub irradiance_voxel_offsets: HashMap<AssetId<IrradianceVoxels>, GpuArrayBufferIndex<u32>>,
}

pub struct IrradianceVolumeInstance {
    pub transform: Mat4,
    pub voxels: AssetId<IrradianceVoxels>,
}

pub struct IrradianceVolumesPlugin;

impl AssetLoader for IrradianceVoxelsAssetLoader {
    type Asset = IrradianceVoxels;
    type Settings = ();
    type Error = IrradianceVoxelsAssetLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _: &'a Self::Settings,
        _: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut buffer = vec![];
            reader.read_to_end(&mut buffer).await?;
            let mut buffer = Cursor::new(buffer);

            let mut magic_number = [0; 4];
            buffer.read_exact(&mut magic_number)?;
            if magic_number != *IRRADIANCE_VOXELS_MAGIC_NUMBER {
                return Err(IrradianceVoxelsAssetLoadError::BadMagicNumber);
            }

            let version = buffer.read_u32::<LittleEndian>()?;
            if version != IRRADIANCE_VOXELS_VERSION {
                return Err(IrradianceVoxelsAssetLoadError::BadVersion);
            }

            let resolution = uvec3(
                buffer.read_u32::<LittleEndian>()?,
                buffer.read_u32::<LittleEndian>()?,
                buffer.read_u32::<LittleEndian>()?,
            );

            let mut data = vec![];
            for _ in 0..(resolution.x * resolution.y * resolution.z * 6) {
                data.push(buffer.read_u32::<LittleEndian>()?);
            }

            Ok(IrradianceVoxels { resolution, data })
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: [&str; 1] = [IRRADIANCE_VOXELS_EXTENSION];
        &EXTENSIONS
    }
}

impl Plugin for IrradianceVolumesPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            IRRADIANCE_VOLUMES_SHADER_HANDLE,
            "irradiance_volume.wgsl",
            Shader::from_wgsl
        );

        app.init_asset::<IrradianceVoxels>()
            .init_asset_loader::<IrradianceVoxelsAssetLoader>()
            .add_plugins(ExtractInstancesPlugin::<IrradianceVolumeInstance>::new())
            .add_plugins(RenderAssetPlugin::<IrradianceVoxels>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            prepare_irradiance_volumes.in_set(RenderSet::PrepareResources),
        );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<RenderIrradianceVolumes>();
    }
}

pub fn prepare_irradiance_volumes(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut render_irradiance_volumes: ResMut<RenderIrradianceVolumes>,
    irradiance_volumes: Res<ExtractedInstances<IrradianceVolumeInstance>>,
    irradiance_voxel_assets: Res<RenderAssets<IrradianceVoxels>>,
) {
    // Do we need to upload new irradiance volumes?
    let active_irradiance_volumes: HashSet<_> = irradiance_volumes
        .iter()
        .filter_map(|(_, instance)| {
            if irradiance_voxel_assets.get(instance.voxels).is_some() {
                Some(instance.voxels)
            } else {
                None
            }
        })
        .collect();
    let irradiance_volumes_dirty = active_irradiance_volumes.len()
        != render_irradiance_volumes.irradiance_voxel_offsets.len()
        || active_irradiance_volumes.iter().any(|irradiance_volume| {
            !render_irradiance_volumes
                .irradiance_voxel_offsets
                .contains_key(irradiance_volume)
        });

    // Upload all irradiance volumes if necessary.
    // FIXME(pcwalton): Don't do this every frame!
    //if irradiance_volumes_dirty {
    render_irradiance_volumes.gpu_irradiance_volumes.clear();
    render_irradiance_volumes.irradiance_voxel_offsets.clear();

    let mut irradiance_volume_data_len = 0;
    for (_, instance) in irradiance_volumes.iter() {
        let Some(irradiance_volume) = irradiance_voxel_assets.get(instance.voxels) else {
            continue;
        };

        let mut start_index = None;
        for &voxel in &irradiance_volume.data {
            let index = render_irradiance_volumes.gpu_irradiance_volumes.push(voxel);
            if start_index.is_none() {
                start_index = Some(index);
            }
            irradiance_volume_data_len += 1;
        }

        render_irradiance_volumes.irradiance_voxel_offsets.insert(
            instance.voxels,
            start_index.unwrap_or(GpuArrayBufferIndex {
                index: 0u32.try_into().unwrap(),
                dynamic_offset: None,
                element_type: PhantomData,
            }),
        );
    }

    // We need at least one element in the buffer for the binding to be valid.
    if irradiance_volume_data_len == 0 {
        render_irradiance_volumes.gpu_irradiance_volumes.push(0);
    }

    render_irradiance_volumes
        .gpu_irradiance_volumes
        .write_buffer(&render_device, &render_queue);
    //}

    // Loop over all irradiance volumes in the scene and update the metadata.
    let mut irradiance_volume_count = 0;
    for (_, instance) in irradiance_volumes.iter() {
        let Some(irradiance_volume) = irradiance_voxel_assets.get(instance.voxels) else {
            continue;
        };

        let transform = instance.transform;

        render_irradiance_volumes
            .gpu_irradiance_volume_metadata
            .get_mut()
            .volumes[irradiance_volume_count] = GpuIrradianceVolume {
            transform,
            inverse_transform: transform.inverse(),
            resolution: irradiance_volume.resolution,
            start_offset: render_irradiance_volumes.irradiance_voxel_offsets[&instance.voxels]
                .index
                .get(),
        };

        irradiance_volume_count += 1;
    }

    render_irradiance_volumes
        .gpu_irradiance_volume_metadata
        .write_buffer(&render_device, &render_queue);
}

impl FromWorld for RenderIrradianceVolumes {
    fn from_world(world: &mut World) -> Self {
        let gpu_irradiance_volume_metadata = UniformBuffer::from_world(world);
        let render_device = world.resource::<RenderDevice>();
        RenderIrradianceVolumes {
            gpu_irradiance_volume_metadata,
            gpu_irradiance_volumes: GpuArrayBuffer::new(&render_device),
            irradiance_voxel_offsets: HashMap::new(),
        }
    }
}

impl Default for GpuIrradianceVolumes {
    fn default() -> Self {
        Self {
            volumes: [GpuIrradianceVolume::default(); MAX_IRRADIANCE_VOLUMES],
        }
    }
}

impl RenderAsset for IrradianceVoxels {
    type PreparedAsset = IrradianceVoxels;
    type Param = ();

    fn persistence_policy(&self) -> RenderAssetPersistencePolicy {
        RenderAssetPersistencePolicy::Keep
    }

    fn prepare_asset(
        self,
        _: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
        // FIXME(pcwalton): Double-cloning seems suboptimal. Wrap in an Arc.
        Ok(self.clone())
    }
}

impl ExtractInstance for IrradianceVolumeInstance {
    type Data = (Read<GlobalTransform>, Read<IrradianceVolume>);
    type Filter = ();

    fn extract((global_transform, irradiance_volume): QueryItem<'_, Self::Data>) -> Option<Self> {
        Some(IrradianceVolumeInstance {
            transform: global_transform.compute_matrix(),
            voxels: irradiance_volume.voxels.id(),
        })
    }
}
