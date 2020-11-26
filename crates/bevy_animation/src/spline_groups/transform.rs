use crate::{
    spline_group::{LoopStyle, SplineGroup},
    vec3_option::Vec3Option,
};
use bevy_math::Quat;
use splines::{Interpolation, Key, Spline};

/// A wrapper for a `Quat`, which represents the orientation of an animation keyframe.
#[derive(Clone, Copy)]
#[repr(transparent)]
struct SlerpWrapper(Quat);

impl std::ops::Add<Self> for SlerpWrapper {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl std::ops::Sub<Self> for SlerpWrapper {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 * other.0.conjugate())
    }
}

impl splines::interpolate::Linear<f32> for SlerpWrapper {
    fn outer_mul(self, t: f32) -> Self {
        Self(Quat::identity().slerp(self.0, t))
    }

    fn outer_div(self, t: f32) -> Self {
        self.outer_mul(1.0 / t)
    }
}

impl splines::Interpolate<f32> for SlerpWrapper {
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

impl Into<Quat> for SlerpWrapper {
    fn into(self) -> Quat {
        self.0
    }
}

impl AsRef<Quat> for SlerpWrapper {
    fn as_ref(&self) -> &Quat {
        &self.0
    }
}

impl AsMut<Quat> for SlerpWrapper {
    fn as_mut(&mut self) -> &mut Quat {
        &mut self.0
    }
}

pub trait SplineQuatExt {
    fn slerp(self) -> Self;
}

impl SplineQuatExt for Spline<f32, Quat> {
    fn slerp(self) -> Self {
        let keys = self.keys();

        if keys.len() < 2 {
            return self;
        }

        let mut new_keys = Vec::with_capacity(keys.len());
        for window in keys.windows(2) {
            // TODO array_windows
            let [a, b] = match window {
                &[a, b] => [a, b],
                _ => unreachable!(),
            };

            match a.interpolation {
                Interpolation::Step(_) => new_keys.push(a),
                Interpolation::Linear => {
                    new_keys.push(a);

                    const COS_MIN_ANGLE: f32 = 0.9995;
                    let cos_angle = a.value.dot(b.value);
                    if cos_angle < COS_MIN_ANGLE {
                        let angle = cos_angle.acos();
                        let min_angle = COS_MIN_ANGLE.acos();
                        let steps = (angle / min_angle) as u32 + 1;

                        let step_t = (b.t - a.t) / steps as f32;
                        for i in 1..steps {
                            let delta_t = (i as f32) * step_t;
                            new_keys.push(Key {
                                value: a.value.slerp(b.value, delta_t),
                                t: delta_t + a.t,
                                interpolation: Interpolation::Linear,
                            });
                        }
                    }
                }
                _ => unimplemented!("Only Interpolation::Linear and Step are supported for now."),
            }
        }
        new_keys.push(*keys.last().unwrap());

        Spline::from_vec(new_keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_resample() {
        const MIN_ANGLE: f32 = 0.0316;
        let spline = Spline::from_vec(vec![
            Key {
                t: 0.,
                value: Quat::from_axis_angle(bevy_math::Vec3::unit_z(), 0.),
                interpolation: Interpolation::Linear,
            },
            Key {
                t: 1.,
                value: Quat::from_axis_angle(bevy_math::Vec3::unit_z(), MIN_ANGLE),
                interpolation: Interpolation::Linear,
            },
            Key {
                t: 2.,
                value: Quat::from_axis_angle(bevy_math::Vec3::unit_z(), 2. * MIN_ANGLE),
                interpolation: Interpolation::Linear,
            },
        ]);

        assert_eq!(spline.clone().keys(), spline.slerp().keys());
    }

    #[test]
    fn big_resample() {
        let spline = Spline::from_vec(vec![
            Key {
                t: 0.,
                value: Quat::from_xyzw(0., 0., 0., 1.),
                interpolation: Interpolation::Linear,
            },
            Key {
                t: 1.,
                value: Quat::from_xyzw(0., 0., 1., 0.),
                interpolation: Interpolation::Linear,
            },
        ]);

        let start_time = |spline: &Spline<f32, Quat>| spline.keys().first().unwrap().t;
        let end_time = |spline: &Spline<f32, Quat>| spline.keys().last().unwrap().t;

        let slerped = spline.clone().slerp();

        assert_eq!(start_time(&spline), start_time(&slerped));
        assert_eq!(end_time(&spline), end_time(&slerped));
        assert!(spline.keys().len() < slerped.keys().len());
    }
}

pub struct TransformSample {
    pub translation: Vec3Option,
    pub rotation: Option<Quat>,
    pub scale: Option<f32>,
}

pub struct AnimationSplineTransform {
    pub translation_x: Spline<f32, f32>,
    pub translation_y: Spline<f32, f32>,
    pub translation_z: Spline<f32, f32>,
    pub rotation: Spline<f32, Quat>,
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
            rotation: self.rotation.sample(time),
            scale: self.scale.sample(time),
        }
    }
}
