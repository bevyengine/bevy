use crate::animatable::{Animatable, Splines};
use bevy_ecs::Component;

pub enum AnimationLoop {
    Once,
    Loop,
    PingPong,
}

pub struct Animator<C: Animatable + Component> {
    pub property: String,
    pub time: f32,
    pub speed: f32,
    pub direction: AnimationLoop,
    pub splines: C::Splines,
    pub pong: bool,
    pub component: std::marker::PhantomData<C>,
}

impl<C: Animatable + Component> Default for Animator<C> {
    fn default() -> Self {
        Self {
            property: "".to_string(),
            time: 0.0,
            speed: 1.0,
            direction: AnimationLoop::Once,
            splines: C::Splines::default(),
            pong: false,
            component: std::marker::PhantomData,
        }
    }
}

impl<C: Animatable + Component> Animator<C> {
    pub fn current(&self) -> Vec<Option<f32>> {
        self.sample(self.time)
    }
    pub fn sample(&self, time: f32) -> Vec<Option<f32>> {
        self.splines.vec().iter().map(|s| s.sample(time)).collect()
    }
    pub fn update_value(&self, component: &mut C) {
        let mut values = component.values();
        for (i, new_val) in self.sample(self.time).iter().enumerate() {
            if let Some(new_val) = new_val {
                values[i] = *new_val;
            }
        }
        component.set_values(values);
    }
    pub fn progress(&mut self, component: &mut C, delta: core::time::Duration) {
        if self.is_empty() {
            return;
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

        match self.direction {
            AnimationLoop::Once => {
                if !past_boundary {
                    self.time += delta.as_secs_f32() * self.speed;
                    self.update_value(component);
                }
            }
            AnimationLoop::Loop => {
                if !past_boundary {
                    self.time += delta.as_secs_f32() * self.speed;
                    self.update_value(component);
                } else {
                    self.time = loop_time_start;
                }
            }
            AnimationLoop::PingPong => {
                if !past_boundary {
                    self.time += delta.as_secs_f32() * self.speed * pong_signum;
                    self.update_value(component);
                } else {
                    self.pong = !self.pong;
                    self.time = if self.pong { end } else { start };
                }
            }
        };
    }
    pub fn duration(&self) -> Option<f32> {
        let start = self.start_time();
        let end = self.end_time();
        if start.is_none() || end.is_none() {
            None
        } else {
            Some((self.start_time().unwrap() - self.end_time().unwrap()).abs())
        }
    }
    pub fn start_time(&self) -> Option<f32> {
        let keys = self.splines.vec();
        if keys.is_empty() {
            return None;
        }
        let mut first_keys = vec![];
        for spline in self.splines.vec() {
            if spline.len() > 0 {
                first_keys.push(spline.get(0).unwrap());
            }
        }

        let mut smallest = first_keys.get(0).unwrap().t;
        for k in first_keys {
            if k.t < smallest {
                smallest = k.t;
            }
        }

        Some(smallest)
    }
    pub fn end_time(&self) -> Option<f32> {
        let keys = self.splines.vec();
        if keys.is_empty() {
            return None;
        }
        let mut last_keys = vec![];
        for spline in self.splines.vec() {
            if spline.len() > 0 {
                last_keys.push(spline.get(spline.len() - 1).unwrap());
            }
        }

        let mut largest = last_keys.get(0).unwrap().t;
        for k in last_keys {
            if k.t > largest {
                largest = k.t;
            }
        }

        Some(largest)
    }

    /// Returns true if all splines are empty
    pub fn is_empty(&self) -> bool {
        self.splines.vec().iter().fold(
            true,
            |acc, spline| {
                if spline.is_empty() {
                    acc
                } else {
                    false
                }
            },
        )
    }
}
