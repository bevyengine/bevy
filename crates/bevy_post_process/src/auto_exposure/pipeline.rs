use super::compensation_curve::{
    AutoExposureCompensationCurve, AutoExposureCompensationCurveUniform,
};
use bevy_asset::{load_embedded_asset, prelude::*};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{binding_types::*, *},
    view::ViewUniform,
};
use bevy_shader::Shader;
use bevy_utils::default;
use core::num::NonZero;

#[derive(Resource)]
pub struct AutoExposurePipeline {
    pub histogram_layout: BindGroupLayoutDescriptor,
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
pub struct AutoExposureUniform {
    pub(super) min_log_lum: f32,
    pub(super) inv_log_lum_range: f32,
    pub(super) log_lum_range: f32,
    pub(super) low_percent: f32,
    pub(super) high_percent: f32,
    pub(super) speed_up: f32,
    pub(super) speed_down: f32,
    pub(super) exponential_transition_distance: f32,
    pub(super) correction_min: f32,
    pub(super) correction_max: f32,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum AutoExposurePass {
    Histogram,
    Average,
}

pub const HISTOGRAM_BIN_COUNT: u64 = 64;

pub fn init_auto_exposure_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AutoExposurePipeline {
        histogram_layout: BindGroupLayoutDescriptor::new(
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
        ),
        histogram_shader: load_embedded_asset!(asset_server.as_ref(), "auto_exposure.wesl"),
    });
}

impl SpecializedComputePipeline for AutoExposurePipeline {
    type Key = AutoExposurePass;

    fn specialize(&self, pass: AutoExposurePass) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("luminance compute pipeline".into()),
            layout: vec![self.histogram_layout.clone()],
            shader: self.histogram_shader.clone(),
            shader_defs: vec![],
            entry_point: Some(match pass {
                AutoExposurePass::Histogram => "compute_histogram".into(),
                AutoExposurePass::Average => "compute_average".into(),
            }),
            ..default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AutoExposureUniform;
    use bevy_render::render_resource::{encase, ShaderType};

    #[test]
    fn uniform_serialization_layout() {
        // The shader's AutoExposure struct contains ten 4-byte scalars.
        assert_eq!(AutoExposureUniform::min_size().get(), 40);

        let uniform = AutoExposureUniform {
            min_log_lum: -8.0,
            inv_log_lum_range: 1.0 / 16.0,
            log_lum_range: 16.0,
            low_percent: 0.1,
            high_percent: 0.9,
            speed_up: 3.0,
            speed_down: 1.0,
            exponential_transition_distance: 1.5,
            correction_min: -2.0,
            correction_max: 3.0,
        };
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&uniform).unwrap();
        let bytes = buffer.into_inner();
        assert_eq!(bytes.len(), 40);
        assert_eq!(&bytes[32..36], &(-2.0f32).to_le_bytes());
        assert_eq!(&bytes[36..40], &3.0f32.to_le_bytes());
    }
}
