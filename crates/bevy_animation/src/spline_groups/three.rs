use crate::{
    spline_group::{LoopStyle, SplineGroup},
    vec3_option::Vec3Option,
};
use splines::Spline;

pub struct AnimationSplineThree {
    pub x: Spline<f32, f32>,
    pub y: Spline<f32, f32>,
    pub z: Spline<f32, f32>,
    pub loop_style: LoopStyle,
    pub time: f32,
    pub speed: f32,
    pub paused: bool,
    pub pong: bool,
}

impl Default for AnimationSplineThree {
    fn default() -> Self {
        Self {
            x: Spline::from_vec(vec![]),
            y: Spline::from_vec(vec![]),
            z: Spline::from_vec(vec![]),
            loop_style: LoopStyle::Once,
            time: 0.0,
            speed: 1.0,

            paused: false,
            pong: false,
        }
    }
}

impl SplineGroup for AnimationSplineThree {
    type Sample = Vec3Option;

    fn spline_key_times(&self) -> Vec<Box<dyn DoubleEndedIterator<Item = f32> + '_>> {
        vec![
            Box::new(self.x.keys().iter().map(|key| key.t)),
            Box::new(self.y.keys().iter().map(|key| key.t)),
            Box::new(self.z.keys().iter().map(|key| key.t)),
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
        Vec3Option::new(
            self.x.sample(time),
            self.y.sample(time),
            self.z.sample(time),
        )
    }
}
