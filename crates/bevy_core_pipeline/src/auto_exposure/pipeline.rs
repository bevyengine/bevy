use super::compensation_curve::{
    AutoExposureCompensationCurve, AutoExposureCompensationCurveUniform,
};
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{binding_types::*, *},
    renderer::RenderDevice,
    texture::Image,
    view::ViewUniform,
};
use std::num::NonZeroU64;

#[derive(Resource)]
pub struct AutoExposurePipeline {
    pub histogram_layout: BindGroupLayout,
    pub histogram_shader: Handle<Shader>,
}

#[derive(Component)]
pub struct ViewAutoExposurePipeline {
    pub histogram_pipeline: CachedComputePipelineId,
    pub mean_luminance_pipeline: CachedComputePipelineId,
    pub compensation_curve: Handle<AutoExposureCompensationCurve>,
    pub metering_mask: Handle<Image>,
}

#[derive(ShaderType, Clone, Copy)]
pub struct AutoExposureSettingsUniform {
    pub(super) min_log_lum: f32,
    pub(super) inv_log_lum_range: f32,
    pub(super) log_lum_range: f32,
    pub(super) low_percent: f32,
    pub(super) high_percent: f32,
    pub(super) speed_up: f32,
    pub(super) speed_down: f32,
    pub(super) exponential_transition_distance: f32,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum AutoExposurePass {
    Histogram,
    Average,
}

pub const METERING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(12987620402995522466);

pub const HISTOGRAM_BIN_COUNT: u64 = 64;

impl FromWorld for AutoExposurePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        Self {
            histogram_layout: render_device.create_bind_group_layout(
                "compute histogram bind group",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        uniform_buffer::<GlobalsUniform>(false),
                        uniform_buffer::<AutoExposureSettingsUniform>(false),
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        texture_1d(TextureSampleType::Float { filterable: false }),
                        uniform_buffer::<AutoExposureCompensationCurveUniform>(false),
                        storage_buffer_sized(false, NonZeroU64::new(HISTOGRAM_BIN_COUNT * 4)),
                        storage_buffer_sized(false, NonZeroU64::new(4)),
                        storage_buffer::<ViewUniform>(true),
                    ),
                ),
            ),
            histogram_shader: METERING_SHADER_HANDLE.clone(),
        }
    }
}

impl SpecializedComputePipeline for AutoExposurePipeline {
    type Key = AutoExposurePass;

    fn specialize(&self, pass: AutoExposurePass) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("luminance compute pipeline".into()),
            layout: vec![self.histogram_layout.clone()],
            shader: self.histogram_shader.clone(),
            shader_defs: vec![],
            entry_point: match pass {
                AutoExposurePass::Histogram => "compute_histogram".into(),
                AutoExposurePass::Average => "compute_average".into(),
            },
            push_constant_ranges: vec![],
        }
    }
}
