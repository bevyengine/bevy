use crate::vec3_option::Vec3Option;
use splines::Spline;

pub enum LoopStyle {
    Once,
    Loop,
    PingPong,
}

pub struct AnimationSpline {
    pub spline: Spline<f32, f32>,
    pub loop_style: LoopStyle,
    pub time: f32,
    pub speed: f32,
    pub pong: bool,
}

impl Default for AnimationSpline {
    fn default() -> Self {
        Self {
            spline: Spline::from_vec(vec![]),
            loop_style: LoopStyle::Once,
            time: 0.0,
            speed: 1.0,
            pong: false,
        }
    }
}

impl AnimationSpline {
    pub fn sample(&self, time: f32) -> Option<f32> {
        self.spline.sample(time)
    }
    pub fn current(&self) -> Option<f32> {
        self.spline.sample(self.time)
    }
    pub fn is_empty(&self) -> bool {
        self.spline.is_empty()
    }
    pub fn start_time(&self) -> Option<f32> {
        spline_start_time(&self.spline)
    }
    pub fn end_time(&self) -> Option<f32> {
        spline_end_time(&self.spline)
    }
    pub fn duration(&self) -> Option<f32> {
        let start = self.start_time();
        let end = self.end_time();
        if start.is_none() || end.is_none() {
            None
        } else {
            Some((start.unwrap() - end.unwrap()).abs())
        }
    }
    pub fn advance(&mut self, delta_time: f32) -> Option<f32> {
        if self.is_empty() {
            return None;
        }

        let start = self.start_time().unwrap();
        let end = self.end_time().unwrap();
        let direction = self.speed.signum();
        let reversed = direction < 0.0;
        let past_boundary = if reversed {
            if self.pong {
                end < self.time
            } else {
                start > self.time
            }
        } else {
            if self.pong {
                start > self.time
            } else {
                end < self.time
            }
        };

        let loop_time_start = if reversed { end } else { start };
        let pong_signum = if self.pong { -1.0 } else { 1.0 };

        match self.loop_style {
            LoopStyle::Once => {
                if !past_boundary {
                    self.time += delta_time * self.speed;
                }
            }
            LoopStyle::Loop => {
                if !past_boundary {
                    self.time += delta_time * self.speed;
                } else {
                    self.time = loop_time_start;
                }
            }
            LoopStyle::PingPong => {
                if !past_boundary {
                    self.time += delta_time * self.speed * pong_signum;
                } else {
                    self.pong = !self.pong;
                    self.time = if self.pong { end } else { start };
                }
            }
        }

        self.current()
    }
}

pub struct AnimationSplineThree {
    pub x: Spline<f32, f32>,
    pub y: Spline<f32, f32>,
    pub z: Spline<f32, f32>,
    pub loop_style: LoopStyle,
    pub time: f32,
    pub speed: f32,
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
            pong: false,
        }
    }
}

impl AnimationSplineThree {
    pub fn sample(&self, time: f32) -> Vec3Option {
        Vec3Option::new(
            self.x.sample(time),
            self.y.sample(time),
            self.z.sample(time),
        )
    }
    pub fn current(&self) -> Vec3Option {
        self.sample(self.time)
    }
    pub fn is_empty(&self) -> bool {
        self.x.is_empty() && self.y.is_empty() && self.z.is_empty()
    }
    pub fn start_time(&self) -> Option<f32> {
        let starts = vec![
            spline_start_time(&self.x),
            spline_start_time(&self.y),
            spline_start_time(&self.z),
        ];
        let starts: Vec<f32> = starts.iter().filter_map(|s| *s).collect();
        if starts.is_empty() {
            None
        } else {
            Some(
                starts
                    .iter()
                    .fold(starts[0], |acc, v| if *v < acc { *v } else { acc }),
            )
        }
    }
    pub fn end_time(&self) -> Option<f32> {
        let starts = vec![
            spline_end_time(&self.x),
            spline_end_time(&self.y),
            spline_end_time(&self.z),
        ];
        let starts: Vec<f32> = starts.iter().filter_map(|s| *s).collect();
        if starts.is_empty() {
            None
        } else {
            Some(
                starts
                    .iter()
                    .fold(starts[0], |acc, v| if *v < acc { *v } else { acc }),
            )
        }
    }
    pub fn duration(&self) -> Option<f32> {
        let start = self.start_time();
        let end = self.end_time();
        if start.is_none() || end.is_none() {
            None
        } else {
            Some((start.unwrap() - end.unwrap()).abs())
        }
    }
    pub fn advance(&mut self, delta_time: f32) -> Vec3Option {
        if self.is_empty() {
            return Vec3Option::none();
        }

        let start = self.start_time().unwrap();
        let end = self.end_time().unwrap();
        let direction = self.speed.signum();
        let reversed = direction < 0.0;
        let past_boundary = if reversed {
            if self.pong {
                end < self.time
            } else {
                start > self.time
            }
        } else {
            if self.pong {
                start > self.time
            } else {
                end < self.time
            }
        };

        let loop_time_start = if reversed { end } else { start };
        let pong_signum = if self.pong { -1.0 } else { 1.0 };

        match self.loop_style {
            LoopStyle::Once => {
                if !past_boundary {
                    self.time += delta_time * self.speed;
                }
            }
            LoopStyle::Loop => {
                if !past_boundary {
                    self.time += delta_time * self.speed;
                } else {
                    self.time = loop_time_start;
                }
            }
            LoopStyle::PingPong => {
                if !past_boundary {
                    self.time += delta_time * self.speed * pong_signum;
                } else {
                    self.pong = !self.pong;
                    self.time = if self.pong { end } else { start };
                }
            }
        }

        self.current()
    }
}

fn spline_start_time(spline: &Spline<f32, f32>) -> Option<f32> {
    if spline.is_empty() {
        return None;
    }
    if let Some(first_key) = spline.get(0) {
        Some(first_key.t)
    } else {
        None
    }
}

fn spline_end_time(spline: &Spline<f32, f32>) -> Option<f32> {
    if spline.is_empty() {
        return None;
    }
    if let Some(last_key) = spline.get(spline.len() - 1) {
        Some(last_key.t)
    } else {
        None
    }
}
