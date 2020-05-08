// pathfinder/renderer/src/gpu/debug.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A debug overlay.
//!
//! We don't render the demo UI text using Pathfinder itself so that we can use the debug UI to
//! debug Pathfinder if it's totally busted.
//!
//! The debug font atlas was generated using: https://evanw.github.io/font-texture-generator/

use crate::gpu::renderer::{RenderStats, RenderTime};
use pathfinder_geometry::vector::{Vector2I, vec2i};
use pathfinder_geometry::rect::RectI;
use pathfinder_gpu::Device;
use pathfinder_resources::ResourceLoader;
use pathfinder_ui::{FONT_ASCENT, LINE_HEIGHT, PADDING, UIPresenter, WINDOW_COLOR};
use std::collections::VecDeque;
use std::ops::{Add, Div};
use std::time::Duration;

const SAMPLE_BUFFER_SIZE: usize = 60;

const STATS_WINDOW_WIDTH: i32 = 325;
const STATS_WINDOW_HEIGHT: i32 = LINE_HEIGHT * 4 + PADDING + 2;

const PERFORMANCE_WINDOW_WIDTH: i32 = 400;
const PERFORMANCE_WINDOW_HEIGHT: i32 = LINE_HEIGHT * 4 + PADDING + 2;

pub struct DebugUIPresenter<D>
where
    D: Device,
{
    pub ui_presenter: UIPresenter<D>,
    cpu_samples: SampleBuffer<RenderStats>,
    gpu_samples: SampleBuffer<RenderTime>,
}

impl<D> DebugUIPresenter<D>
where
    D: Device,
{
    pub fn new(
        device: &D,
        resources: &dyn ResourceLoader,
        framebuffer_size: Vector2I,
    ) -> DebugUIPresenter<D> {
        let ui_presenter = UIPresenter::new(device, resources, framebuffer_size);
        DebugUIPresenter {
            ui_presenter,
            cpu_samples: SampleBuffer::new(),
            gpu_samples: SampleBuffer::new(),
        }
    }

    pub fn add_sample(&mut self, stats: RenderStats, rendering_time: RenderTime) {
        self.cpu_samples.push(stats);
        self.gpu_samples.push(rendering_time);
    }

    pub fn draw(&self, device: &D) {
        self.draw_stats_window(device);
        self.draw_performance_window(device);
    }

    fn draw_stats_window(&self, device: &D) {
        let framebuffer_size = self.ui_presenter.framebuffer_size();
        let bottom = framebuffer_size.y() - PADDING;
        let window_rect = RectI::new(
            vec2i(framebuffer_size.x() - PADDING - STATS_WINDOW_WIDTH,
                  bottom - PERFORMANCE_WINDOW_HEIGHT - PADDING - STATS_WINDOW_HEIGHT),
            vec2i(STATS_WINDOW_WIDTH, STATS_WINDOW_HEIGHT),
        );

        self.ui_presenter.draw_solid_rounded_rect(device, window_rect, WINDOW_COLOR);

        let mean_cpu_sample = self.cpu_samples.mean();
        let origin = window_rect.origin() + vec2i(PADDING, PADDING + FONT_ASCENT);
        self.ui_presenter.draw_text(
            device,
            &format!("Paths: {}", mean_cpu_sample.path_count),
            origin,
            false,
        );
        self.ui_presenter.draw_text(
            device,
            &format!("Solid Tiles: {}", mean_cpu_sample.solid_tile_count),
            origin + vec2i(0, LINE_HEIGHT * 1),
            false,
        );
        self.ui_presenter.draw_text(
            device,
            &format!("Alpha Tiles: {}", mean_cpu_sample.alpha_tile_count),
            origin + vec2i(0, LINE_HEIGHT * 2),
            false,
        );
        self.ui_presenter.draw_text(
            device,
            &format!("Fills: {}", mean_cpu_sample.fill_count),
            origin + vec2i(0, LINE_HEIGHT * 3),
            false,
        );
    }

    fn draw_performance_window(&self, device: &D) {
        let framebuffer_size = self.ui_presenter.framebuffer_size();
        let bottom = framebuffer_size.y() - PADDING;
        let window_rect = RectI::new(
            vec2i(framebuffer_size.x() - PADDING - PERFORMANCE_WINDOW_WIDTH,
                  bottom - PERFORMANCE_WINDOW_HEIGHT),
            vec2i(PERFORMANCE_WINDOW_WIDTH, PERFORMANCE_WINDOW_HEIGHT),
        );

        self.ui_presenter.draw_solid_rounded_rect(device, window_rect, WINDOW_COLOR);

        let mean_cpu_sample = self.cpu_samples.mean();
        let origin = window_rect.origin() + vec2i(PADDING, PADDING + FONT_ASCENT);
        self.ui_presenter.draw_text(
            device,
            &format!("CPU: {:.3} ms", duration_to_ms(mean_cpu_sample.cpu_build_time)),
            origin,
            false,
        );

        let mean_gpu_sample = self.gpu_samples.mean();
        self.ui_presenter.draw_text(
            device,
            &format!("GPU: {:.3} ms", duration_to_ms(mean_gpu_sample.gpu_time)),
            origin + vec2i(0, LINE_HEIGHT * 1),
            false,
        );

        let wallclock_time = f64::max(duration_to_ms(mean_gpu_sample.gpu_time),
                                      duration_to_ms(mean_cpu_sample.cpu_build_time));
        self.ui_presenter.draw_text(
            device,
            &format!("Wallclock: {:.3} ms", wallclock_time),
            origin + vec2i(0, LINE_HEIGHT * 3),
            false,
        );
    }

}

struct SampleBuffer<S>
where
    S: Add<S, Output = S> + Div<usize, Output = S> + Clone + Default,
{
    samples: VecDeque<S>,
}

impl<S> SampleBuffer<S>
where
    S: Add<S, Output = S> + Div<usize, Output = S> + Clone + Default,
{
    fn new() -> SampleBuffer<S> {
        SampleBuffer {
            samples: VecDeque::with_capacity(SAMPLE_BUFFER_SIZE),
        }
    }

    fn push(&mut self, time: S) {
        self.samples.push_back(time);
        while self.samples.len() > SAMPLE_BUFFER_SIZE {
            self.samples.pop_front();
        }
    }

    fn mean(&self) -> S {
        let mut mean = Default::default();
        if self.samples.is_empty() {
            return mean;
        }

        for time in &self.samples {
            mean = mean + (*time).clone();
        }

        mean / self.samples.len()
    }
}

#[derive(Clone, Default)]
struct CPUSample {
    elapsed: Duration,
    stats: RenderStats,
}

impl Add<CPUSample> for CPUSample {
    type Output = CPUSample;
    fn add(self, other: CPUSample) -> CPUSample {
        CPUSample {
            elapsed: self.elapsed + other.elapsed,
            stats: self.stats + other.stats,
        }
    }
}

impl Div<usize> for CPUSample {
    type Output = CPUSample;
    fn div(self, divisor: usize) -> CPUSample {
        CPUSample {
            elapsed: self.elapsed / (divisor as u32),
            stats: self.stats / divisor,
        }
    }
}

fn duration_to_ms(time: Duration) -> f64 {
    time.as_secs() as f64 * 1000.0 + time.subsec_nanos() as f64 / 1000000.0
}
