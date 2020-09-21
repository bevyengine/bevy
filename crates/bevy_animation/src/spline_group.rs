use core::time::Duration;
use splines::Spline;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopStyle {
    Once,
    Loop,
    PingPong,
}

pub trait SplineGroup {
    type Sample;

    fn splines(&self) -> Vec<&Spline<f32, f32>>;

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

    fn is_empty(&self) -> bool {
        let any_not_empty = self.splines().into_iter().any(|v| !v.is_empty());
        !any_not_empty
    }

    fn start_time(&self) -> Option<f32> {
        let starts: Vec<f32> = self
            .splines()
            .into_iter()
            .filter_map(spline_start_time)
            .collect();

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

    fn end_time(&self) -> Option<f32> {
        let ends: Vec<f32> = self
            .splines()
            .into_iter()
            .map(|s| spline_end_time(s))
            .filter_map(|s| s)
            .collect();

        if ends.is_empty() {
            None
        } else {
            Some(
                ends.iter()
                    .fold(ends[0], |acc, v| if *v > acc { *v } else { acc }),
            )
        }
    }

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

fn spline_start_time(spline: &Spline<f32, f32>) -> Option<f32> {
    spline.get(0).map(|first_key| first_key.t)
}

fn spline_end_time(spline: &Spline<f32, f32>) -> Option<f32> {
    spline
        .get(spline.len().saturating_sub(1))
        .map(|last_key| last_key.t)
}
