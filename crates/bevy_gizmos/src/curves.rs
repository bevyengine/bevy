//! Additional [`GizmoBuffer`] Functions -- Curves
//!
//! Includes the implementation of [`GizmoBuffer::curve_2d`],
//! [`GizmoBuffer::curve_3d`] and assorted support items.

use bevy_color::Color;
use bevy_math::{
    curve::{Curve, CurveExt},
    Vec2, Vec3,
};

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw a curve, at the given time points, sampling in 2D.
    ///
    /// This should be called for each frame the curve needs to be rendered.
    ///
    /// Samples of time points outside of the curve's domain will be filtered out and won't
    /// contribute to the rendering. If you wish to render the curve outside of its domain you need
    /// to create a new curve with an extended domain.
    ///
    /// # Arguments
    /// - `curve_2d` some type that implements the [`Curve`] trait and samples `Vec2`s
    /// - `times` some iterable type yielding `f32` which will be used for sampling the curve
    /// - `color` the color of the curve
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED};
    /// fn system(mut gizmos: Gizmos) {
    ///     let domain = Interval::UNIT;
    ///     let curve = FunctionCurve::new(domain, |t| Vec2::from(t.sin_cos()));
    ///     gizmos.curve_2d(curve, (0..=100).map(|n| n as f32 / 100.0), RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn curve_2d(
        &mut self,
        curve_2d: impl Curve<Vec2>,
        times: impl IntoIterator<Item = f32>,
        color: impl Into<Color>,
    ) {
        self.linestrip_2d(curve_2d.sample_iter(times).flatten(), color);
    }

    /// Draw a curve, at the given time points, sampling in 3D.
    ///
    /// This should be called for each frame the curve needs to be rendered.
    ///
    /// Samples of time points outside of the curve's domain will be filtered out and won't
    /// contribute to the rendering. If you wish to render the curve outside of its domain you need
    /// to create a new curve with an extended domain.
    ///
    /// # Arguments
    /// - `curve_3d` some type that implements the [`Curve`] trait and samples `Vec3`s
    /// - `times` some iterable type yielding `f32` which will be used for sampling the curve
    /// - `color` the color of the curve
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED};
    /// fn system(mut gizmos: Gizmos) {
    ///     let domain = Interval::UNIT;
    ///     let curve = FunctionCurve::new(domain, |t| {
    ///         let (x,y) = t.sin_cos();
    ///         Vec3::new(x, y, t)
    ///     });
    ///     gizmos.curve_3d(curve, (0..=100).map(|n| n as f32 / 100.0), RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn curve_3d(
        &mut self,
        curve_3d: impl Curve<Vec3>,
        times: impl IntoIterator<Item = f32>,
        color: impl Into<Color>,
    ) {
        self.linestrip(curve_3d.sample_iter(times).flatten(), color);
    }

    /// Draw a curve, at the given time points, sampling in 2D, with a color gradient.
    ///
    /// This should be called for each frame the curve needs to be rendered.
    ///
    /// Samples of time points outside of the curve's domain will be filtered out and won't
    /// contribute to the rendering. If you wish to render the curve outside of its domain you need
    /// to create a new curve with an extended domain.
    ///
    /// # Arguments
    /// - `curve_2d` some type that implements the [`Curve`] trait and samples `Vec2`s
    /// - `times_with_colors` some iterable type yielding `f32` which will be used for sampling
    ///   the curve together with the color at this position
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::{Mix, palettes::basic::{GREEN, RED}};
    /// fn system(mut gizmos: Gizmos) {
    ///     let domain = Interval::UNIT;
    ///     let curve = FunctionCurve::new(domain, |t| Vec2::from(t.sin_cos()));
    ///     gizmos.curve_gradient_2d(
    ///         curve,
    ///         (0..=100).map(|n| n as f32 / 100.0)
    ///                  .map(|t| (t, GREEN.mix(&RED, t)))
    ///     );
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn curve_gradient_2d<C>(
        &mut self,
        curve_2d: impl Curve<Vec2>,
        times_with_colors: impl IntoIterator<Item = (f32, C)>,
    ) where
        C: Into<Color>,
    {
        self.linestrip_gradient_2d(
            times_with_colors
                .into_iter()
                .filter_map(|(time, color)| curve_2d.sample(time).map(|sample| (sample, color))),
        );
    }

    /// Draw a curve, at the given time points, sampling in 3D, with a color gradient.
    ///
    /// This should be called for each frame the curve needs to be rendered.
    ///
    /// Samples of time points outside of the curve's domain will be filtered out and won't
    /// contribute to the rendering. If you wish to render the curve outside of its domain you need
    /// to create a new curve with an extended domain.
    ///
    /// # Arguments
    /// - `curve_3d` some type that implements the [`Curve`] trait and samples `Vec3`s
    /// - `times_with_colors` some iterable type yielding `f32` which will be used for sampling
    ///   the curve together with the color at this position
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::{Mix, palettes::basic::{GREEN, RED}};
    /// fn system(mut gizmos: Gizmos) {
    ///     let domain = Interval::UNIT;
    ///     let curve = FunctionCurve::new(domain, |t| {
    ///         let (x,y) = t.sin_cos();
    ///         Vec3::new(x, y, t)
    ///     });
    ///     gizmos.curve_gradient_3d(
    ///         curve,
    ///         (0..=100).map(|n| n as f32 / 100.0)
    ///                  .map(|t| (t, GREEN.mix(&RED, t)))
    ///     );
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn curve_gradient_3d<C>(
        &mut self,
        curve_3d: impl Curve<Vec3>,
        times_with_colors: impl IntoIterator<Item = (f32, C)>,
    ) where
        C: Into<Color>,
    {
        self.linestrip_gradient(
            times_with_colors
                .into_iter()
                .filter_map(|(time, color)| curve_3d.sample(time).map(|sample| (sample, color))),
        );
    }
}
