use super::compensation_curve::{
    AutoExposureCompensationCurve, AutoExposureCompensationCurveUniform,
};
use bevy_asset::{load_embedded_asset, prelude::*};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{binding_types::*, *},
    renderer::RenderDevice,
    view::ViewUniform,
};
use bevy_utils::default;
use core::{num::NonZero, result::Result};

#[derive(Resource)]
pub struct AutoExposurePipeline {
    pub layout: BindGroupLayout,
    pub variants: SpecializedCache<ComputePipeline, AutoExposureSpecializer>,
}

pub struct AutoExposureSpecializer;

#[derive(Component)]
pub struct ViewAutoExposurePipeline {
    pub histogram_pipeline: CachedComputePipelineId,
    pub mean_luminance_pipeline: CachedComputePipelineId,
    pub compensation_curve: Handle<AutoExposureCompensationCurve>,
    pub metering_mask: Handle<Image>,
}

#[derive(ShaderType, Clone, Copy)]
pub struct AutoExposureUniform {
    pub(super) min_log_lum: f32,
    pub(super) inv_log_lum_range: f32,
    pub(super) log_lum_range: f32,
    pub(super) low_percent: f32,
    pub(super) high_percent: f32,
    pub(super) speed_up: f32,
    pub(super) speed_down: f32,
    pub(super) exponential_transition_distance: f32,
}

#[derive(PartialEq, Eq, Hash, Clone, SpecializerKey)]
pub enum AutoExposurePass {
    Histogram,
    Average,
}

pub const HISTOGRAM_BIN_COUNT: u64 = 64;

impl FromWorld for AutoExposurePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "compute histogram bind group",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<GlobalsUniform>(false),
                    uniform_buffer::<AutoExposureUniform>(false),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_1d(TextureSampleType::Float { filterable: false }),
                    uniform_buffer::<AutoExposureCompensationCurveUniform>(false),
                    storage_buffer_sized(false, NonZero::<u64>::new(HISTOGRAM_BIN_COUNT * 4)),
                    storage_buffer_sized(false, NonZero::<u64>::new(4)),
                    storage_buffer::<ViewUniform>(true),
                ),
            ),
        );

        let shader = load_embedded_asset!(world, "auto_exposure.wgsl");

        let base_descriptor = ComputePipelineDescriptor {
            layout: vec![layout.clone()],
            shader,
            ..default()
        };

        let variants = SpecializedCache::new(AutoExposureSpecializer, None, base_descriptor);

        Self { layout, variants }
    }
}

impl Specializer<ComputePipeline> for AutoExposureSpecializer {
    type Key = AutoExposurePass;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut ComputePipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        let (label, entry_point) = match key {
            AutoExposurePass::Histogram => (
                "auto_exposure_compute_histogram".into(),
                "compute_histogram".into(),
            ),
            AutoExposurePass::Average => (
                "auto_exposure_compute_average".into(),
                "compute_average".into(),
            ),
        };

        descriptor.label = Some(label);
        descriptor.entry_point = Some(entry_point);

        Ok(key)
    }
}
