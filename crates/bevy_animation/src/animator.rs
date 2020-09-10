use crate::animatable::{AnimTracks, Animatable};
use bevy_ecs::Component;
use bevy_ecs::Mut;
use splines::{Key, Spline};

type AnimatableSpline<T: Animatable> = Spline<f32, T::Track>;

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
    pub splines: Vec<AnimatableSpline<C>>,
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
            splines: {
                let mut splines = vec![];
                for _ in 0..C::anim_tracks().len() {
                    splines.push(Spline::from_vec(vec![]));
                }
                splines
            },
            pong: false,
            component: std::marker::PhantomData,
        }
    }
}

impl<C: Animatable + Component> Animator<C> {
    pub fn sample(&self, time: f32) -> Vec<Option<C::Track>> {
        self.splines.iter().map(|s| s.sample(time)).collect()
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
        // if self.is_empty() {
        //     return;
        // }

        // let direction = self.speed.signum();
        // let reversed = direction < 0.0;
        // let past_boundary = if reversed {
        //     self.start_time().unwrap() > self.time
        // } else {
        //     self.end_time().unwrap() < self.time
        // };

        match self.direction {
            AnimationLoop::Once => {
                self.time += delta.as_secs_f32() * self.speed;
            }
            AnimationLoop::Loop => {
                todo!();
            }
            AnimationLoop::PingPong => {
                todo!();
            }
        };
        self.update_value(component);
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
        let first_keys: Vec<&Key<f32, C::Track>> = self
            .splines
            .iter()
            .map(|s| s.keys().get(0))
            .filter(|k| k.is_some())
            .map(|s| s.unwrap())
            .collect();

        if first_keys.is_empty() {
            return None;
        }

        let smallest = first_keys
            .iter()
            .fold(first_keys.get(0).unwrap().t, |acc, key| {
                if key.t < acc {
                    key.t
                } else {
                    acc
                }
            });

        Some(smallest)
    }
    pub fn end_time(&self) -> Option<f32> {
        let last_keys: Vec<&Key<f32, C::Track>> = self
            .splines
            .iter()
            .map(|s| s.keys().get(s.keys().len() - 1))
            .filter(|k| k.is_some())
            .map(|s| s.unwrap())
            .collect();

        if last_keys.is_empty() {
            return None;
        }

        let largest = last_keys
            .iter()
            .fold(last_keys.get(0).unwrap().t, |acc, key| {
                if key.t > acc {
                    key.t
                } else {
                    acc
                }
            });

        Some(largest)
    }

    /// Returns true if all splines are empty
    pub fn is_empty(&self) -> bool {
        self.splines.iter().fold(
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
