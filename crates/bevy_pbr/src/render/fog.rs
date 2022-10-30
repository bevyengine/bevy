use bevy_ecs::{
    schedule::SystemLabel,
    system::{Res, ResMut, Resource},
};
use bevy_math::Vec4;
use bevy_render::{
    render_resource::{ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
};

use crate::{Fog, FogMode};

/// The GPU-side representation of the fog configuration that's sent as a uniform to the shader
#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuFog {
    /// unsigned int representation of the active fog mode
    mode: u32,
    /// fog color
    color: Vec4,
    /// for linear fog, `start`; for other modes of fog, `density`.
    density_or_start: f32,
    /// for linear fog, `end`; for other modes of fog, unused
    end: f32,
}

const GPU_FOG_MODE_OFF: u32 = 0;
const GPU_FOG_MODE_LINEAR: u32 = 1;
const GPU_FOG_MODE_EXPONENTIAL: u32 = 2;
const GPU_FOG_MODE_EXPONENTIAL_SQUARED: u32 = 3;

/// Metadata for fog
#[derive(Default, Resource)]
pub struct FogMeta {
    pub gpu_fog: UniformBuffer<GpuFog>,
}

/// Prepares fog metadata and writes the fog-related uniform buffers to the GPU
pub fn prepare_fog(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut fog_meta: ResMut<FogMeta>,
    fog: Res<Fog>,
) {
    let gpu_fog = match &fog.mode {
        FogMode::Off => GpuFog {
            mode: GPU_FOG_MODE_OFF,
            ..Default::default()
        },
        FogMode::Linear { start, end } => GpuFog {
            mode: GPU_FOG_MODE_LINEAR,
            color: fog.color.into(),
            density_or_start: *start,
            end: *end,
        },
        FogMode::Exponential { density } => GpuFog {
            mode: GPU_FOG_MODE_EXPONENTIAL,
            color: fog.color.into(),
            density_or_start: *density,
            ..Default::default()
        },
        FogMode::ExponentialSquared { density } => GpuFog {
            mode: GPU_FOG_MODE_EXPONENTIAL_SQUARED,
            color: fog.color.into(),
            density_or_start: *density,
            ..Default::default()
        },
    };

    fog_meta.gpu_fog.set(gpu_fog);
    fog_meta.gpu_fog.write_buffer(&render_device, &render_queue);
}

/// Labels for fog-related systems
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum RenderFogSystems {
    PrepareFog,
}
