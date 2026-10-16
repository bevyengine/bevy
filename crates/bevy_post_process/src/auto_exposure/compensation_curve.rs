use bevy_asset::{prelude::*, RenderAssetUsages};
use bevy_curve::cubic_splines::CubicGenerator;
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_math::{FloatExt, Vec2};
use bevy_reflect::prelude::*;
use bevy_render::{
    render_asset::{AssetExtractionError, RenderAsset},
    render_resource::{
        Extent3d, ShaderType, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        TextureView, UniformBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
};
use thiserror::Error;

const LUT_SIZE: usize = 256;

/// An auto exposure compensation curve.
/// This curve is used to map the average luminance of a scene, in EV100, to an
/// exposure compensation value, to allow for fine control over the final exposure.
#[derive(Asset, Reflect, Debug, Clone)]
#[reflect(Default, Clone)]
pub struct AutoExposureCompensationCurve {
    /// The minimum scene luminance in the curve, in EV100. (the x-axis)
    min_ev100: f32,
    /// The maximum scene luminance in the curve, in EV100. (the x-axis)
    max_ev100: f32,
    /// The minimum exposure compensation value in the curve. (the y-axis)
    min_compensation: f32,
    /// The maximum exposure compensation value in the curve. (the y-axis)
    max_compensation: f32,
    /// The lookup table for the curve. Uploaded to the GPU as a 1D texture.
    /// Each value in the LUT is a `u8` representing a normalized exposure compensation value:
    /// * `0` maps to `min_compensation`
    /// * `255` maps to `max_compensation`
    ///
    /// The position in the LUT corresponds to the normalized scene luminance.
    /// * `0` maps to `min_ev100`
    /// * `LUT_SIZE - 1` maps to `max_ev100`
    lut: [u8; LUT_SIZE],
}

/// Various errors that can occur when constructing an [`AutoExposureCompensationCurve`].
#[derive(Error, Debug)]
pub enum AutoExposureCompensationCurveError {
    /// The curve couldn't be built in the first place.
    #[error("curve could not be constructed from the given data")]
    InvalidCurve,
    /// A discontinuity was found in the curve.
    #[error("discontinuity found between curve segments")]
    DiscontinuityFound,
    /// The curve is not monotonically increasing on the x-axis.
    #[error("curve is not monotonically increasing on the x-axis")]
    NotMonotonic,
}

impl Default for AutoExposureCompensationCurve {
    fn default() -> Self {
        Self {
            min_ev100: 0.0,
            max_ev100: 0.0,
            min_compensation: 0.0,
            max_compensation: 0.0,
            lut: [0; LUT_SIZE],
        }
    }
}

impl AutoExposureCompensationCurve {
    const SAMPLES_PER_SEGMENT: usize = 64;

    /// Build an [`AutoExposureCompensationCurve`] from a [`CubicGenerator<Vec2>`], where:
    /// - x represents the average luminance of the scene in EV100 (see
    ///   [`AutoExposure::range`](super::AutoExposure::range));
    /// - y represents the exposure compensation value in F-stops (see
    ///   [`AutoExposure::compensation_curve`](super::AutoExposure::compensation_curve)).
    ///
    /// # Errors
    ///
    /// If the curve is not monotonically increasing on the x-axis,
    /// returns [`AutoExposureCompensationCurveError::NotMonotonic`].
    ///
    /// If a discontinuity is found between curve segments,
    /// returns [`AutoExposureCompensationCurveError::DiscontinuityFound`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_asset::prelude::*;
    /// # use bevy_math::vec2;
    /// # use bevy_curve::cubic_splines::*;
    /// # use bevy_post_process::auto_exposure::AutoExposureCompensationCurve;
    /// # let mut compensation_curves = Assets::<AutoExposureCompensationCurve>::default();
    /// let curve: Handle<AutoExposureCompensationCurve> = compensation_curves.add(
    ///     AutoExposureCompensationCurve::from_curve(LinearSpline::new([
    ///         vec2(9.0, -2.0),
    ///         vec2(13.0, 0.0),
    ///         vec2(15.0, 0.0),
    ///         vec2(17.0, 2.0),
    ///     ]))
    ///     .unwrap()
    /// );
    /// ```
    pub fn from_curve<T>(curve: T) -> Result<Self, AutoExposureCompensationCurveError>
    where
        T: CubicGenerator<Vec2>,
    {
        let Ok(curve) = curve.to_curve() else {
            return Err(AutoExposureCompensationCurveError::InvalidCurve);
        };

        let min_ev100 = curve.position(0.0).x;
        let max_ev100 = curve.position(curve.segments().len() as f32).x;
        let ev100_range = max_ev100 - min_ev100;

        let mut lut = [0.0; LUT_SIZE];

        let mut previous = curve.position(0.0);
        let mut min_compensation = previous.y;
        let mut max_compensation = previous.y;

        for segment in curve {
            if segment.position(0.0) != previous {
                return Err(AutoExposureCompensationCurveError::DiscontinuityFound);
            }

            for i in 1..Self::SAMPLES_PER_SEGMENT {
                let current = segment.position(i as f32 / (Self::SAMPLES_PER_SEGMENT - 1) as f32);

                if current.x < previous.x {
                    return Err(AutoExposureCompensationCurveError::NotMonotonic);
                }

                // Find the range of LUT entries that this line segment covers.
                let (lut_begin, lut_end) = (
                    ((previous.x - min_ev100) / ev100_range) * (LUT_SIZE - 1) as f32,
                    ((current.x - min_ev100) / ev100_range) * (LUT_SIZE - 1) as f32,
                );
                let lut_inv_range = 1.0 / (lut_end - lut_begin);

                // Iterate over all LUT entries whose pixel centers fall within the current segment.
                #[expect(
                    clippy::needless_range_loop,
                    reason = "This for-loop also uses `i` to calculate a value `t`."
                )]
                for i in lut_begin.ceil() as usize..=lut_end.floor() as usize {
                    let t = (i as f32 - lut_begin) * lut_inv_range;
                    lut[i] = previous.y.lerp(current.y, t);
                    min_compensation = min_compensation.min(lut[i]);
                    max_compensation = max_compensation.max(lut[i]);
                }

                previous = current;
            }
        }

        let compensation_range = max_compensation - min_compensation;

        Ok(Self {
            min_ev100,
            max_ev100,
            min_compensation,
            max_compensation,
            lut: if compensation_range > 0.0 {
                let scale = 255.0 / compensation_range;
                lut.map(|f: f32| ((f - min_compensation) * scale) as u8)
            } else {
                [0; LUT_SIZE]
            },
        })
    }
}

/// The GPU-representation of an [`AutoExposureCompensationCurve`].
/// Consists of a [`TextureView`] with the curve's data,
/// and a [`UniformBuffer`] with the curve's extents.
pub struct GpuAutoExposureCompensationCurve {
    pub(super) texture_view: TextureView,
    pub(super) extents: UniformBuffer<AutoExposureCompensationCurveUniform>,
}

#[derive(ShaderType, Clone, Copy)]
pub(super) struct AutoExposureCompensationCurveUniform {
    min_ev100: f32,
    inv_ev100_range: f32,
    min_compensation: f32,
    compensation_range: f32,
}

impl RenderAsset for GpuAutoExposureCompensationCurve {
    type SourceAsset = AutoExposureCompensationCurve;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        _previous_gpu_asset: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        Ok(source.clone())
    }

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_device, render_queue): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, bevy_render::render_asset::PrepareAssetError<Self::SourceAsset>> {
        let texture = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: LUT_SIZE as u32,
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
            &source.lut,
        );

        let texture_view = texture.create_view(&Default::default());

        let mut extents = UniformBuffer::from(AutoExposureCompensationCurveUniform {
            min_ev100: source.min_ev100,
            inv_ev100_range: 1.0 / (source.max_ev100 - source.min_ev100),
            min_compensation: source.min_compensation,
            compensation_range: source.max_compensation - source.min_compensation,
        });

        extents.write_buffer(render_device, render_queue);

        Ok(GpuAutoExposureCompensationCurve {
            texture_view,
            extents,
        })
    }
}
