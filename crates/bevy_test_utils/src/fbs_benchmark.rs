use std::time::{Duration, Instant};

use bevy_ecs::system::Local;
use bevy_log::{info, warn};

/// Welford's method for computing variance.
#[derive(Default, Debug)]
struct Welford {
    n: u64,
    x: f64,
    s2: f64,
}

impl Welford {
    /// Add a new sample.
    fn add(&mut self, x: f64) {
        let new_n = self.n + 1;
        let new_x = self.x + (x - self.x) / (new_n as f64);
        let new_s2 = self.s2 + (x - self.x) * (x - new_x);

        self.n = new_n;
        self.x = new_x;
        self.s2 = new_s2;
    }

    /// Compute standard deviation.
    fn std(&self) -> f64 {
        let var = self.s2 / ((self.n - 1) as f64);
        var.sqrt()
    }

    /// Compute mean.
    fn mean(&self) -> f64 {
        self.x
    }
}

/// Welford's method.
#[derive(Default, Debug)]
pub(crate) struct FpsBenchmarkState {
    last_run: Option<Instant>,
    started: Option<Instant>,
    frames: u64,
    welford: Welford,
}

pub(crate) fn fps_benchmark_system(
    mut state: Local<FpsBenchmarkState>,
    mut local_last_print: Local<Option<Instant>>,
) {
    if local_last_print.is_none() {
        warn!(
            "Benchmark outputs need to be dealt with carefully!\n\
            First, algorithm assumes normal distribution of frame times, which is not true.\n\"\
            Second, program performance varies a lot depending on the environment, for example:\n\
            - CPU frequency is throttled when the CPU is too hot or battery is too low\n\
            - other processes running on the system, like a browser, slow down this application\n\
            - render iteration is faster when window is not visible\n\
            - random program state like heap fragmentation or hash collisions\n\
              systematically affect performance and this is not corrected\n\
              by running the benchmark for a long time"
        );
    }

    state.frames += 1;
    let now = Instant::now();
    if let (Some(last_run), Some(started)) = (state.last_run, state.started) {
        // Do not update statistics while the application is warming up.
        // Output would be converge if we did, but it would take longer to converge,
        // and statistics would be less accurate.
        if now - started >= Duration::from_secs(1) && state.frames >= 100 {
            let x = (now - last_run).as_secs_f64();
            state.welford.add(x);
        }
    }
    state.last_run = Some(now);
    if state.started.is_none() {
        state.started = Some(now);
    }

    if let Some(last_print) = *local_last_print {
        if (now - last_print).as_secs_f64() >= 1.0 && state.welford.n >= 2 {
            let n = state.welford.n;

            info!("frame count: {}", n);

            let frame_time_mean = state.welford.mean();
            let frame_time_std = state.welford.std();
            let frame_time_error = frame_time_std / (n as f64).sqrt();
            // Multiply by 3 to get 99.7% confidence interval.
            // But careful, this is assuming normal distribution of frame times,
            // which is not true.
            info!(
                "frame time : {:>5}us +- {:>4}us",
                (frame_time_mean * 1_000_000.0) as u64,
                (frame_time_error * 1_000_000.0 * 3.0) as u64
            );

            // Now computing error of FPS.
            // We can reasonably assume than mean of FPS is inverse of mean of frame duration.
            // (Which is, again, not true, because frame time distribution is skewed.)
            let fps_mean = 1.0 / frame_time_mean;

            // Standard deviation of f(x) is approximately f'(mean(x)) * std(x).
            // f(x) = 1 / x
            // f'(x) = -1 / x^2
            // std(f) = (1 / mean(x)^2) * std(x)
            let fps_std = frame_time_std / frame_time_mean.powi(2);

            // And now compute the error of FPS.
            let fps_error = fps_std / (n as f64).sqrt();

            // See above about multiplying by 3.
            info!("fps        : {:7.3} +- {:.3}", fps_mean, fps_error * 3.0);
            *local_last_print = Some(now);
        }
    } else {
        *local_last_print = Some(now);
    }
}
