use bevy_color::Color;
use bevy_math::{curve::Curve, Vec2, Vec3};

use crate::prelude::{GizmoConfigGroup, Gizmos};

impl<'w, 's, Config, Clear> Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    pub fn curve_2d(
        &mut self,
        curve_2d: impl Curve<Vec2>,
        times: impl IntoIterator<Item = f32>,
        color: impl Into<Color>,
    ) {
        self.linestrip_2d(curve_2d.sample_iter(times).flatten(), color);
    }

    pub fn curve_3d(
        &mut self,
        curve_3d: impl Curve<Vec3>,
        times: impl IntoIterator<Item = f32>,
        color: impl Into<Color>,
    ) {
        self.linestrip(curve_3d.sample_iter(times).flatten(), color);
    }

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
