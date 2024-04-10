use bevy_asset::prelude::*;
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_math::{
    cubic_splines::{CubicCurve, CubicSegment},
    Vec2,
};
use bevy_reflect::prelude::*;
use bevy_render::{
    render_asset::{RenderAsset, RenderAssetUsages},
    render_resource::{
        encase, Buffer, BufferInitDescriptor, BufferUsages, Extent3d, ShaderType,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    },
    renderer::{RenderDevice, RenderQueue},
};

/// An auto exposure compensation curve.
/// This curve is used to map the average log luminance of a scene to an
/// exposure compensation value, to allow for fine control over the final exposure.
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect(Default)]
pub struct AutoExposureCompensationCurve {
    min_log_lum: f32,
    max_log_lum: f32,
    min_compensation: f32,
    max_compensation: f32,
    data: [u8; 256],
}

impl Default for AutoExposureCompensationCurve {
    fn default() -> Self {
        Self {
            min_log_lum: 0.0,
            max_log_lum: 0.0,
            min_compensation: 0.0,
            max_compensation: 0.0,
            data: [0; 256],
        }
    }
}

impl From<CubicCurve<Vec2>> for AutoExposureCompensationCurve {
    /// Constructs a new [`AutoExposureCompensationCurve`] from a [`CubicCurve<Vec2>`], where:
    /// - x represents the average log luminance of the scene in EV-100;
    /// - y represents the exposure compensation value in F-stops.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_asset::prelude::*;
    /// # use bevy_math::vec2;
    /// # use bevy_math::cubic_splines::*;
    /// # use bevy_core_pipeline::auto_exposure::AutoExposureCompensationCurve;
    /// # let mut compensation_curves = Assets::<AutoExposureCompensationCurve>::default();
    /// let curve: Handle<AutoExposureCompensationCurve> = compensation_curves.add(
    ///    LinearSpline::new([
    ///        vec2(-4.0, -2.0),
    ///        vec2(0.0, 0.0),
    ///        vec2(2.0, 0.0),
    ///        vec2(4.0, 2.0),
    ///    ])
    ///    .to_curve()
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the end of a segment is not equal to the beginning of the next segment,
    /// or if the curve is not monotonically increasing on the x-axis.
    fn from(curve: CubicCurve<Vec2>) -> Self {
        let min_log_lum = curve.position(0.0).x;
        let max_log_lum = curve.position(curve.segments().len() as f32).x;
        let domain = max_log_lum - min_log_lum;
        let step = domain / 255.0;
        assert!(min_log_lum < max_log_lum);

        let mut data = [0.0; 256];
        let mut min_compensation = std::f32::MAX;
        let mut max_compensation = std::f32::MIN;
        let mut previous_end = min_log_lum;

        for segment in curve {
            let begin = segment.position(0.0).x;
            let end = segment.position(1.0).x;
            assert!(begin < end);
            assert!(begin == previous_end);
            previous_end = end;

            let i = (((begin - min_log_lum) / domain) * 255.0).ceil() as usize;
            let j = (((end - min_log_lum) / domain) * 255.0).floor() as usize;

            for (k, v) in data[i..=j].iter_mut().enumerate() {
                *v = find_y_given_x(&segment, min_log_lum + (i + k) as f32 * step);
                min_compensation = v.min(min_compensation);
                max_compensation = v.max(max_compensation);
            }
        }

        let compensation_range = max_compensation - min_compensation;

        Self {
            min_log_lum,
            max_log_lum,
            min_compensation,
            max_compensation,
            data: if compensation_range > 0.0 {
                data.map(|f: f32| {
                    (((f - min_compensation) / compensation_range) * 255.0).clamp(0.0, 255.0) as u8
                })
            } else {
                [0; 256]
            },
        }
    }
}

/// Maximum allowable error for iterative Bezier solve
const MAX_ERROR: f32 = 1e-5;

/// Maximum number of iterations during Bezier solve
const MAX_ITERS: u8 = 8;

/// Find the `y` value of the curve at the given `x` value using the Newton-Raphson method.
fn find_y_given_x(segment: &CubicSegment<Vec2>, x: f32) -> f32 {
    let mut t_guess = x;
    let mut pos_guess = Vec2::ZERO;
    for _ in 0..MAX_ITERS {
        pos_guess = segment.position(t_guess);
        let error = pos_guess.x - x;
        if error.abs() <= MAX_ERROR {
            break;
        }
        // Using Newton's method, use the tangent line to estimate a better guess value.
        let slope = segment.velocity(t_guess).x; // dx/dt
        t_guess -= error / slope;
    }
    pos_guess.y
}

/// The GPU-representation of an [`AutoExposureCompensationCurve`].
/// Consists of a [`TextureView`] with the curve's data, and a [`Buffer`] with the curve's extents.
#[derive(Debug, Clone)]
pub struct GpuAutoExposureCompensationCurve {
    pub(super) texture_view: TextureView,
    pub(super) extents: Buffer,
}

#[derive(ShaderType, Clone, Copy)]
pub(super) struct AutoExposureCompensationCurveUniform {
    min_log_lum: f32,
    inv_log_lum_range: f32,
    min_compensation: f32,
    compensation_range: f32,
}

impl RenderAsset for GpuAutoExposureCompensationCurve {
    type SourceAsset = AutoExposureCompensationCurve;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        source: Self::SourceAsset,
        (render_device, render_queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, bevy_render::render_asset::PrepareAssetError<Self::SourceAsset>> {
        let texture = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 256,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D1,
                format: TextureFormat::R8Unorm,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[TextureFormat::R8Unorm],
            },
            Default::default(),
            &source.data,
        );

        let texture_view = texture.create_view(&Default::default());

        let mut settings = encase::UniformBuffer::new(Vec::new());
        settings
            .write(&AutoExposureCompensationCurveUniform {
                min_log_lum: source.min_log_lum,
                inv_log_lum_range: 1.0 / (source.max_log_lum - source.min_log_lum),
                min_compensation: source.min_compensation,
                compensation_range: source.max_compensation - source.min_compensation,
            })
            .unwrap();

        let extents = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: settings.as_ref(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Ok(GpuAutoExposureCompensationCurve {
            texture_view,
            extents,
        })
    }
}
