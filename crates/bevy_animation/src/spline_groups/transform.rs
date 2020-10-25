use bevy_math::Quat;
use crate::{
    spline_group::{LoopStyle, SplineGroup},
    vec3_option::Vec3Option,
};
use splines::Spline;
use std::fmt;

pub struct TransformSample {
    pub translation: Vec3Option,
    pub rotation: Option<Quat>,
    pub scale: Option<f32>,
}

/// A wrapper for a `Quat`, which represents the orientation of an animation keyframe.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rotation(Quat);

impl Rotation {
    /// Wrap a `Quat` as a `Rotation`.
    pub fn new(q: Quat) -> Self {
        q.into()
    }
}

impl std::ops::Add<Self> for Rotation {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl std::ops::Sub<Self> for Rotation {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 * other.0.conjugate())
    }
}

impl splines::interpolate::Linear<f32> for Rotation {
    fn outer_mul(self, t: f32) -> Self {
        Self(Quat::identity().slerp(self.0, t))
    }

    fn outer_div(self, t: f32) -> Self {
        self.outer_mul(1.0 / t)
    }
}

impl splines::Interpolate<f32> for Rotation {
    fn lerp(Self(a): Self, Self(b): Self, t: f32) -> Self {
        Self(a.slerp(b, t))
    }

    fn quadratic_bezier(_: Self, _: Self, _: Self, _: f32) -> Self {
        todo!()
    }

    fn cubic_bezier(_: Self, _: Self, _: Self, _: Self, _: f32) -> Self {
        todo!()
    }
}

impl fmt::Display for Rotation {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

impl From<Quat> for Rotation {
    fn from(q: Quat) -> Self {
        Self(q)
    }
}

impl Into<Quat> for Rotation {
    fn into(self) -> Quat {
        self.0
    }
}

impl AsRef<Quat> for Rotation {
    fn as_ref(&self) -> &Quat {
        &self.0
    }
}

impl AsMut<Quat> for Rotation {
    fn as_mut(&mut self) -> &mut Quat {
        &mut self.0
    }
}

pub struct AnimationSplineTransform {
    pub translation_x: Spline<f32, f32>,
    pub translation_y: Spline<f32, f32>,
    pub translation_z: Spline<f32, f32>,
    pub rotation: Spline<f32, Rotation>,
    pub scale: Spline<f32, f32>,
    pub loop_style: LoopStyle,
    pub time: f32,
    pub speed: f32,
    pub paused: bool,
    pub pong: bool,
}

impl Default for AnimationSplineTransform {
    fn default() -> Self {
        Self {
            translation_x: Spline::from_vec(vec![]),
            translation_y: Spline::from_vec(vec![]),
            translation_z: Spline::from_vec(vec![]),
            rotation: Spline::from_vec(vec![]),
            scale: Spline::from_vec(vec![]),
            loop_style: LoopStyle::Once,
            time: 0.0,
            speed: 1.0,
            paused: false,
            pong: false,
        }
    }
}

impl SplineGroup for AnimationSplineTransform {
    type Sample = TransformSample;

    fn spline_key_times(&self) -> Vec<Box<dyn DoubleEndedIterator<Item = f32> + '_>> {
        vec![
            Box::new(self.translation_x.keys().iter().map(|key| key.t)),
            Box::new(self.translation_y.keys().iter().map(|key| key.t)),
            Box::new(self.translation_z.keys().iter().map(|key| key.t)),
            Box::new(self.rotation.keys().iter().map(|key| key.t)),
            Box::new(self.scale.keys().iter().map(|key| key.t)),
        ]
    }

    fn loop_style(&self) -> LoopStyle {
        self.loop_style
    }

    fn loop_style_mut(&mut self) -> &mut LoopStyle {
        &mut self.loop_style
    }

    fn time(&self) -> f32 {
        self.time
    }

    fn time_mut(&mut self) -> &mut f32 {
        &mut self.time
    }

    fn speed(&self) -> f32 {
        self.speed
    }

    fn speed_mut(&mut self) -> &mut f32 {
        &mut self.speed
    }

    fn paused(&self) -> bool {
        self.paused
    }

    fn paused_mut(&mut self) -> &mut bool {
        &mut self.paused
    }

    fn pong(&self) -> bool {
        self.pong
    }

    fn pong_mut(&mut self) -> &mut bool {
        &mut self.pong
    }

    fn sample(&self, time: f32) -> Self::Sample {
        TransformSample {
            translation: Vec3Option::new(
                self.translation_x.sample(time),
                self.translation_y.sample(time),
                self.translation_z.sample(time),
            ),
            rotation: self.rotation.sample(time).map(|o| o.0),
            scale: self.scale.sample(time),
        }
    }
}
