//! A module for the [`AnimatedGizmos`] [`SystemParam`].

use std::ops::{Deref, DerefMut};

use bevy_color::Color;
use bevy_ecs::system::{Res, SystemParam};
use bevy_math::Vec3;
use bevy_time::Time;

use crate::prelude::{DefaultGizmoConfigGroup, GizmoConfigGroup, Gizmos};

/// A [`SystemParam`] for drawing animated gizmos.
///
/// This is basically just a utility wrapper around [`Gizmos`], so for more details take a look
/// at the docs there.
///
/// Note that you can still draw the regular non-animated geometries as with regular [`Gizmos`]
#[derive(SystemParam)]
pub struct AnimatedGizmos<'w, 's, Config = DefaultGizmoConfigGroup, Clear = ()>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: Gizmos<'w, 's, Config, Clear>,
    time: Res<'w, Time>,
}

impl<'w, 's, Config, Clear> Deref for AnimatedGizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type Target = Gizmos<'w, 's, Config, Clear>;
    fn deref(&self) -> &Self::Target {
        &self.gizmos
    }
}

impl<'w, 's, Config, Clear> DerefMut for AnimatedGizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.gizmos
    }
}

/// A builder returned by [`AnimatedGizmos::animated_line`].
pub struct AnimatedLineBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut AnimatedGizmos<'w, 's, Config, Clear>,
    // start position of the animated line
    start: Vec3,
    // end position of the animated line
    end: Vec3,
    // color of the animated line
    color: Color,

    // number of segments of the animated line
    segments: usize,
    // speed factor for the animation
    speed: f32,
}

impl<Config, Clear> AnimatedLineBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of animated line segments.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }

    /// Set the speed factor of the animated line.
    pub fn speed(mut self, factor: f32) -> Self {
        self.speed = factor;
        self
    }
}

impl<'w, 's, Config, Clear> AnimatedGizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw an animated line in 3D from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: AnimatedGizmos) {
    ///     gizmos.animated_line(Vec3::ZERO, Vec3::X, GREEN)
    ///           .segments(10)
    ///           .speed(0.5);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn animated_line(
        &mut self,
        start: Vec3,
        end: Vec3,
        color: impl Into<Color>,
    ) -> AnimatedLineBuilder<'_, 'w, 's, Config, Clear> {
        AnimatedLineBuilder {
            gizmos: self,
            start,
            end,
            color: color.into(),

            segments: 5,
            speed: 0.1,
        }
    }
}

impl<Config, Clear> Drop for AnimatedLineBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.gizmos.enabled {
            return;
        }

        let delta_t = self.gizmos.time.elapsed_seconds();
        let n_f32 = self.segments as f32;
        // * 2.0 here since otherwise there would be no gaps
        let seg_length = (n_f32 * 2.0).recip();
        let diff = self.end - self.start;
        let color = self.color;
        (0..=self.segments)
            .map(|n| n as f32 / n_f32)
            .map(|percent| {
                let percent_offset = percent + delta_t * self.speed;
                // range 0.0..=(N+1)/N
                // -> line transitions out of visible range smoothly
                let modulo = 1.0 + n_f32.recip();
                let percent_final = percent_offset % modulo;
                // range (-1/N)..=(N+1)/N
                // -> line transitions into visible range smoothly
                [(percent_final - seg_length), percent_final]
                    // clamp scalars to be inside the line range
                    .map(|scalar| scalar.clamp(0.0, 1.0))
                    // scalar -> real 3D position
                    .map(|scalar| self.start + scalar * diff)
            })
            .for_each(|[start, end]| {
                self.gizmos.line(start, end, color);
            });
    }
}
