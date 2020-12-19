use core::time::Duration;
use splines::Spline;

pub(crate) trait SplineExt {
    fn start_time(&self) -> Option<f32>;
    fn end_time(&self) -> Option<f32>;
}

impl<V> SplineExt for Spline<f32, V> {
    fn start_time(&self) -> Option<f32> {
        self.keys().first().map(|k| k.t)
    }

    fn end_time(&self) -> Option<f32> {
        self.keys().last().map(|k| k.t)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopStyle {
    Once,
    Loop,
    PingPong,
}

pub trait SplineGroup {
    type Sample;

    fn loop_style(&self) -> LoopStyle;
    fn loop_style_mut(&mut self) -> &mut LoopStyle;

    fn time(&self) -> f32;
    fn time_mut(&mut self) -> &mut f32;

    fn speed(&self) -> f32;
    fn speed_mut(&mut self) -> &mut f32;

    fn paused(&self) -> bool;
    fn paused_mut(&mut self) -> &mut bool;

    fn pong(&self) -> bool;
    fn pong_mut(&mut self) -> &mut bool;

    fn sample(&self, time: f32) -> Self::Sample;

    fn current(&self) -> Self::Sample {
        self.sample(self.time())
    }

    fn is_empty(&self) -> bool;

    fn start_time(&self) -> Option<f32>;

    fn end_time(&self) -> Option<f32>;

    fn duration(&self) -> Option<Duration> {
        self.start_time()
            .zip(self.end_time())
            .map(|(start, end)| (start - end).abs())
            .map(Duration::from_secs_f32)
    }

    fn advance(&mut self, delta_time: f32) {
        if self.is_empty() || self.paused() {
            return;
        }

        let start = self.start_time().unwrap();
        let end = self.end_time().unwrap();
        let direction = self.speed().signum();
        let reversed = direction < 0.0;
        let past_boundary = match (reversed, self.pong()) {
            (true, true) => end < self.time(),
            (true, false) => start > self.time(),
            (false, true) => start > self.time(),
            (false, false) => end < self.time(),
        };

        let loop_time_start = if reversed { end } else { start };
        let pong_signum = if self.pong() { -1.0 } else { 1.0 };

        let speed = self.speed();
        let loop_style = self.loop_style();
        let pong = self.pong();
        let time = self.time_mut();

        let mut new_pong = pong;

        match loop_style {
            LoopStyle::Once => {
                if !past_boundary {
                    *time += delta_time * speed;
                }
            }
            LoopStyle::Loop => {
                if !past_boundary {
                    *time += delta_time * speed;
                } else {
                    *time = loop_time_start;
                }
            }
            LoopStyle::PingPong => {
                if !past_boundary {
                    *time += delta_time * speed * pong_signum;
                } else {
                    new_pong = !pong;
                    *time = if new_pong { end } else { start };
                }
            }
        };

        *self.pong_mut() = new_pong;
    }

    fn pause(&mut self) {
        *self.paused_mut() = true;
    }

    fn play(&mut self) {
        *self.paused_mut() = false;
    }

    fn toggle_pause(&mut self) {
        let paused = self.paused();
        *self.paused_mut() = !paused;
    }
}
