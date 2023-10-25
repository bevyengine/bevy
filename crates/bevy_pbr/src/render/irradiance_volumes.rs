use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetApp, AssetId, Handle};
use bevy_ecs::{
    query::QueryItem,
    schedule::IntoSystemConfigs,
    system::{lifetimeless::Read, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::{IVec3, Mat4, Vec4};
use bevy_render::{
    extract_instances::{ExtractInstance, ExtractInstancesPlugin, ExtractedInstances},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_resource::{GpuArrayBuffer, GpuArrayBufferIndex, Shader, ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{HashMap, HashSet};

use crate::{irradiance_volumes::IrradianceVolume, IrradianceVolumeAssetLoader};

pub const IRRADIANCE_VOLUMES_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(160299515939076705258408299184317675488);

pub const MAX_IRRADIANCE_VOLUMES: usize = 256;

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuIrradianceVolume {
    pub transform: Mat4,
    pub inverse_transform: Mat4,
    pub resolution: IVec3,
    pub start_offset: u32,
}

#[derive(ShaderType)]
pub struct GpuIrradianceVolumes {
    pub volumes: [GpuIrradianceVolume; MAX_IRRADIANCE_VOLUMES],
}

#[derive(Resource)]
pub struct RenderIrradianceVolumes {
    pub gpu_irradiance_volume_metadata: UniformBuffer<GpuIrradianceVolumes>,
    pub gpu_irradiance_volumes: GpuArrayBuffer<Vec4>,
    pub irradiance_volume_offsets: HashMap<AssetId<IrradianceVolume>, GpuArrayBufferIndex<Vec4>>,
}

pub struct IrradianceVolumeInstance {
    pub transform: Mat4,
    pub irradiance_volume: AssetId<IrradianceVolume>,
}

pub struct IrradianceVolumesPlugin;

impl Plugin for IrradianceVolumesPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            IRRADIANCE_VOLUMES_SHADER_HANDLE,
            "irradiance_volumes.wgsl",
            Shader::from_wgsl
        );

        app.init_asset::<IrradianceVolume>()
            .init_asset_loader::<IrradianceVolumeAssetLoader>()
            .add_plugins(ExtractInstancesPlugin::<IrradianceVolumeInstance>::new())
            .add_plugins(RenderAssetPlugin::<IrradianceVolume>::default());

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
    irradiance_volume_assets: Res<RenderAssets<IrradianceVolume>>,
) {
    // Do we need to upload new irradiance volumes?
    let active_irradiance_volumes: HashSet<_> = irradiance_volumes
        .iter()
        .filter_map(|(_, instance)| {
            if irradiance_volume_assets
                .get(instance.irradiance_volume)
                .is_some()
            {
                Some(instance.irradiance_volume)
            } else {
                None
            }
        })
        .collect();
    let irradiance_volumes_dirty = active_irradiance_volumes.len()
        != render_irradiance_volumes.irradiance_volume_offsets.len()
        || active_irradiance_volumes.iter().any(|irradiance_volume| {
            !render_irradiance_volumes
                .irradiance_volume_offsets
                .contains_key(irradiance_volume)
        });

    // Upload all irradiance volumes if necessary.
    // FIXME(pcwalton): Don't do this every frame!
    //if irradiance_volumes_dirty {
    render_irradiance_volumes.gpu_irradiance_volumes.clear();
    render_irradiance_volumes.irradiance_volume_offsets.clear();

    let mut irradiance_volume_data_len = 0;
    for (_, instance) in irradiance_volumes.iter() {
        let Some(irradiance_volume) = irradiance_volume_assets.get(instance.irradiance_volume)
        else {
            continue;
        };

        let mut start_index = None;
        for voxel in &irradiance_volume.data {
            let index = render_irradiance_volumes
                .gpu_irradiance_volumes
                .push(*voxel);
            if start_index.is_none() {
                start_index = Some(index);
            }
            irradiance_volume_data_len += 1;
        }

        render_irradiance_volumes.irradiance_volume_offsets.insert(
            instance.irradiance_volume,
            start_index.unwrap_or(GpuArrayBufferIndex {
                index: 0u32.try_into().unwrap(),
                dynamic_offset: None,
                element_type: PhantomData,
            }),
        );
    }

    // We need at least one element in the buffer for the binding to be valid.
    if irradiance_volume_data_len == 0 {
        render_irradiance_volumes
            .gpu_irradiance_volumes
            .push(Vec4::default());
    }

    render_irradiance_volumes
        .gpu_irradiance_volumes
        .write_buffer(&render_device, &render_queue);
    //}

    // Loop over all irradiance volumes in the scene and update the metadata.
    let mut irradiance_volume_count = 0;
    for (_, instance) in irradiance_volumes.iter() {
        let Some(irradiance_volume) = irradiance_volume_assets.get(instance.irradiance_volume)
        else {
            continue;
        };

        let transform = irradiance_volume.transform.compute_matrix() * instance.transform;

        render_irradiance_volumes
            .gpu_irradiance_volume_metadata
            .get_mut()
            .volumes[irradiance_volume_count] = GpuIrradianceVolume {
            transform,
            inverse_transform: transform.inverse(),
            resolution: irradiance_volume.resolution,
            start_offset: render_irradiance_volumes.irradiance_volume_offsets
                [&instance.irradiance_volume]
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
            irradiance_volume_offsets: HashMap::new(),
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

impl RenderAsset for IrradianceVolume {
    type ExtractedAsset = IrradianceVolume;
    type PreparedAsset = IrradianceVolume;
    type Param = ();

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        irradiance_volume: Self::ExtractedAsset,
        _: &mut bevy_ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        // FIXME(pcwalton): Double-cloning seems suboptimal. Wrap in an Arc.
        Ok(irradiance_volume.clone())
    }
}

impl ExtractInstance for IrradianceVolumeInstance {
    type Query = (Read<GlobalTransform>, Read<Handle<IrradianceVolume>>);
    type Filter = ();

    fn extract((global_transform, irradiance_volume): QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(IrradianceVolumeInstance {
            transform: global_transform.compute_matrix(),
            irradiance_volume: irradiance_volume.id(),
        })
    }
}
