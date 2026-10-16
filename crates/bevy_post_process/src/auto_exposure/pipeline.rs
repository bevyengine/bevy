use super::compensation_curve::{
    AutoExposureCompensationCurve, AutoExposureCompensationCurveUniform,
};
use bevy_asset::{load_embedded_asset, prelude::*};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_render::{
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
    pub(super) min_ev100: f32,
    pub(super) inv_ev100_range: f32,
    pub(super) ev100_range: f32,
    pub(super) low_percent: f32,
    pub(super) high_percent: f32,
    pub(super) speed_brighten: f32,
    pub(super) speed_darken: f32,
    pub(super) exponential_transition_distance: f32,
    pub(super) correction_min: f32,
    pub(super) correction_max: f32,
    /// The camera's `Exposure`, in EV100, which the correction range is relative to.
    pub(super) camera_ev100: f32,
    /// Real time since the previous frame, in seconds.
    pub(super) delta_time: f32,
    /// Nonzero when no pipeline is waiting to compile, or auto exposure has run longer than the
    /// startup timeout. Must stay the last field: the auto exposure pass overwrites it in place.
    pub(super) pipelines_ready: u32,
}

/// The adapted exposure the shader keeps between frames.
#[derive(ShaderType, Clone, Copy, Default)]
pub struct GpuAutoExposureState {
    /// The adapted exposure, in EV100.
    pub(super) ev100: f32,
    /// Zero until the first fully drawn frame with luminance above the bottom of the metering
    /// range sets `ev100` to its target.
    pub(super) initialized: u32,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum AutoExposurePass {
    Histogram,
    Average,
}

/// The number of histogram bins; matches `BIN_COUNT` in `auto_exposure.wesl`.
pub const HISTOGRAM_BIN_COUNT: u64 = 128;

pub fn init_auto_exposure_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AutoExposurePipeline {
        histogram_layout: BindGroupLayoutDescriptor::new(
            "compute histogram bind group",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<AutoExposureUniform>(false),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_1d(TextureSampleType::Float { filterable: false }),
                    uniform_buffer::<AutoExposureCompensationCurveUniform>(false),
                    storage_buffer_sized(false, NonZero::<u64>::new(HISTOGRAM_BIN_COUNT * 4)),
                    storage_buffer::<GpuAutoExposureState>(false),
                    storage_buffer::<ViewUniform>(true),
                    texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
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
    use super::{AutoExposureUniform, GpuAutoExposureState};
    use bevy_render::render_resource::{encase, ShaderType};

    #[test]
    fn uniform_serialization_layout() {
        // The shader's AutoExposure struct contains thirteen 4-byte scalars.
        assert_eq!(AutoExposureUniform::min_size().get(), 52);

        let uniform = AutoExposureUniform {
            min_ev100: -8.0,
            inv_ev100_range: 1.0 / 28.0,
            ev100_range: 28.0,
            low_percent: 0.1,
            high_percent: 0.9,
            speed_brighten: 3.0,
            speed_darken: 1.0,
            exponential_transition_distance: 1.5,
            correction_min: -2.0,
            correction_max: 3.0,
            camera_ev100: 9.7,
            delta_time: 0.25,
            pipelines_ready: 1,
        };
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&uniform).unwrap();
        let bytes = buffer.into_inner();
        assert_eq!(bytes.len(), 52);
        assert_eq!(&bytes[32..36], &(-2.0f32).to_le_bytes());
        assert_eq!(&bytes[36..40], &3.0f32.to_le_bytes());
        assert_eq!(&bytes[40..44], &9.7f32.to_le_bytes());
        assert_eq!(&bytes[44..48], &0.25f32.to_le_bytes());
        assert_eq!(&bytes[48..52], &1u32.to_le_bytes());
    }

    #[test]
    fn state_serialization_layout() {
        // The shader's GpuAutoExposureState is an f32 exposure then a u32 flag.
        assert_eq!(GpuAutoExposureState::min_size().get(), 8);

        let mut buffer = encase::StorageBuffer::new(Vec::<u8>::new());
        buffer
            .write(&GpuAutoExposureState {
                ev100: 9.5,
                initialized: 1,
            })
            .unwrap();
        let bytes = buffer.into_inner();
        assert_eq!(&bytes[0..4], &9.5f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &1u32.to_le_bytes());
    }
}
