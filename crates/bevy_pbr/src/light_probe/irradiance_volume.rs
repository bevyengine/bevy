//! Irradiance volumes.

use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    query::QueryItem,
    schedule::IntoSystemConfigs as _,
    system::{lifetimeless::Read, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    extract_instances::{ExtractInstance, ExtractInstancesPlugin, ExtractedInstances},
    render_asset::RenderAssets,
    render_resource::{Shader, ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    Render, RenderApp, RenderSet,
};
use bevy_transform::components::GlobalTransform;
use std::io;
use thiserror::Error;

use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_math::Mat4;
use bevy_reflect::{Reflect, TypeUuid};

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
    pub voxels: Handle<Image>,
    pub intensity: f32,
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
}

#[derive(ShaderType)]
pub struct GpuIrradianceVolumes {
    pub volumes: [GpuIrradianceVolume; MAX_IRRADIANCE_VOLUMES],
}

#[derive(Resource)]
pub struct RenderIrradianceVolumes {
    pub gpu_irradiance_volume_metadata: UniformBuffer<GpuIrradianceVolumes>,
    pub voxels: Option<AssetId<Image>>,
}

pub struct IrradianceVolumeInstance {
    pub transform: Mat4,
    pub voxels: AssetId<Image>,
}

pub struct IrradianceVolumesPlugin;

impl Plugin for IrradianceVolumesPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            IRRADIANCE_VOLUMES_SHADER_HANDLE,
            "irradiance_volume.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(ExtractInstancesPlugin::<IrradianceVolumeInstance>::new());

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
    image_assets: Res<RenderAssets<Image>>,
) {
    // Loop over all irradiance volumes in the scene and update the metadata.
    let mut irradiance_volume_count = 0;
    for (_, instance) in irradiance_volumes.iter() {
        if image_assets.get(instance.voxels).is_none() {
            continue;
        };

        let transform = instance.transform;

        render_irradiance_volumes
            .gpu_irradiance_volume_metadata
            .get_mut()
            .volumes[irradiance_volume_count] = GpuIrradianceVolume {
            transform,
            inverse_transform: transform.inverse(),
        };

        render_irradiance_volumes.voxels = Some(instance.voxels);

        irradiance_volume_count += 1;
    }

    render_irradiance_volumes
        .gpu_irradiance_volume_metadata
        .write_buffer(&render_device, &render_queue);
}

impl FromWorld for RenderIrradianceVolumes {
    fn from_world(world: &mut World) -> Self {
        let gpu_irradiance_volume_metadata = UniformBuffer::from_world(world);
        RenderIrradianceVolumes {
            gpu_irradiance_volume_metadata,
            voxels: None,
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
