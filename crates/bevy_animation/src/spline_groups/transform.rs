use crate::spline_group::{LoopStyle, SplineGroup};
use crate::vec3_option::Vec3Option;
use splines::Spline;

pub struct TransformSample {
    pub translation: Vec3Option,
    pub rotation: Vec3Option,
    pub scale: Option<f32>,
}

pub struct AnimationSplineTransform {
    pub translation_x: Spline<f32, f32>,
    pub translation_y: Spline<f32, f32>,
    pub translation_z: Spline<f32, f32>,
    pub rotation_x: Spline<f32, f32>,
    pub rotation_y: Spline<f32, f32>,
    pub rotation_z: Spline<f32, f32>,
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
            rotation_x: Spline::from_vec(vec![]),
            rotation_y: Spline::from_vec(vec![]),
            rotation_z: Spline::from_vec(vec![]),
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

    fn splines(&self) -> Vec<&Spline<f32, f32>> {
        vec![
            &self.translation_x,
            &self.translation_y,
            &self.translation_z,
            &self.rotation_x,
            &self.rotation_y,
            &self.rotation_z,
            &self.scale,
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
            rotation: Vec3Option::new(
                self.rotation_x.sample(time),
                self.rotation_y.sample(time),
                self.rotation_z.sample(time),
            ),
            scale: self.scale.sample(time),
        }
    }
}
