use std::time::Duration;

use bevy::{
    animation::{
        interpolate::utils::{Quatx4, Quatx8},
        tracks::{Track, TrackFixed},
        wide::{Vec3x4, Vec3x8, Vec4x4, Vec4x8},
    },
    prelude::*,
};
use rand::prelude::*;

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, Criterion, ParameterizedBenchmark,
};

const LEN: usize = 1_000;
const TICKS: usize = 1_000;
const WARM_UP_TIME: Duration = Duration::from_secs(1);
const MEASUREMENT_TIME: Duration = Duration::from_secs(5);

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "animation/tracks",
        ParameterizedBenchmark::new(
            "4/narrow",
            |b, _| {
                b.iter_batched(
                    BenchNarrow4::setup,
                    |value| {
                        black_box(value.run(black_box(TICKS)));
                    },
                    BatchSize::NumIterations(LEN as u64),
                )
            },
            vec![()],
        )
        .with_function("4/wide", |b, _| {
            b.iter_batched(
                BenchWide4::setup,
                |value| {
                    black_box(value.run(black_box(TICKS)));
                },
                BatchSize::NumIterations(LEN as u64),
            )
        })
        .with_function("8/narrow", |b, _| {
            b.iter_batched(
                BenchNarrow8::setup,
                |value| {
                    black_box(value.run(black_box(TICKS)));
                },
                BatchSize::NumIterations(LEN as u64),
            )
        })
        .with_function("8/wide", |b, _| {
            b.iter_batched(
                BenchWide8::setup,
                |value| {
                    black_box(value.run(black_box(TICKS)));
                },
                BatchSize::NumIterations(LEN as u64),
            )
        })
        .warm_up_time(WARM_UP_TIME)
        .measurement_time(MEASUREMENT_TIME),
    );
}

struct Bench<P, R> {
    t: [f32; 64],
    pos: P,
    rot: R,
}

type BenchNarrow4 = Bench<[TrackFixed<Vec3>; 4], [TrackFixed<Quat>; 4]>;

impl BenchNarrow4 {
    fn setup() -> Self {
        let mut rng = rand::thread_rng();

        let temp = TrackFixed::<Vec3>::from_keyframes(30, 0, vec![]);
        let mut pos = [temp.clone(), temp.clone(), temp.clone(), temp.clone()];

        let temp = TrackFixed::<Quat>::from_keyframes(30, 0, vec![]);
        let mut rot = [temp.clone(), temp.clone(), temp.clone(), temp.clone()];

        for i in 0..4 {
            let mut p: Vec<Vec3> = (0..16).map(|_| Vec3::zero()).collect();
            unsafe {
                rng.fill(std::slice::from_raw_parts_mut(
                    p.as_mut_ptr() as *mut f32,
                    p.len() * 3,
                ));
            }
            let mut temp = TrackFixed::from_keyframes(15, 0, p);
            std::mem::swap(&mut pos[i], &mut temp);

            let mut r: Vec<Quat> = (0..16).map(|_| Quat::identity()).collect();
            unsafe {
                rng.fill(std::slice::from_raw_parts_mut(
                    r.as_mut_ptr() as *mut f32,
                    r.len() * 4,
                ));
            }
            for x in &mut r {
                *x = x.normalize();
            }
            let mut temp = TrackFixed::from_keyframes(15, 0, r);
            std::mem::swap(&mut rot[i], &mut temp);
        }

        let mut t = [0.0; 64];
        rng.fill(&mut t);

        Self { t, pos, rot }
    }

    fn run(&self, iterations: usize) {
        for t in self.t.iter().cycle().take(iterations) {
            for i in 0..4 {
                black_box(self.pos[i].sample(black_box(*t)));
                black_box(self.rot[i].sample(black_box(*t)));
            }
        }
    }
}

type BenchNarrow8 = Bench<[TrackFixed<Vec3>; 8], [TrackFixed<Quat>; 8]>;

impl BenchNarrow8 {
    fn setup() -> Self {
        let mut rng = rand::thread_rng();

        let temp = TrackFixed::<Vec3>::from_keyframes(30, 0, vec![]);
        let mut pos = [
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
        ];

        let temp = TrackFixed::<Quat>::from_keyframes(30, 0, vec![]);
        let mut rot = [
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
            temp.clone(),
        ];

        for i in 0..8 {
            let mut p: Vec<Vec3> = (0..16).map(|_| Vec3::zero()).collect();
            unsafe {
                rng.fill(std::slice::from_raw_parts_mut(
                    p.as_mut_ptr() as *mut f32,
                    p.len() * 3,
                ));
            }
            let mut temp = TrackFixed::from_keyframes(15, 0, p);
            std::mem::swap(&mut pos[i], &mut temp);

            let mut r: Vec<Quat> = (0..16).map(|_| Quat::identity()).collect();
            unsafe {
                rng.fill(std::slice::from_raw_parts_mut(
                    r.as_mut_ptr() as *mut f32,
                    r.len() * 4,
                ));
            }
            for x in &mut r {
                *x = x.normalize();
            }
            let mut temp = TrackFixed::from_keyframes(15, 0, r);
            std::mem::swap(&mut rot[i], &mut temp);
        }

        let mut t = [0.0; 64];
        rng.fill(&mut t);

        Self { t, pos, rot }
    }

    fn run(&self, iterations: usize) {
        for t in self.t.iter().cycle().take(iterations) {
            for i in 0..8 {
                black_box(self.pos[i].sample(black_box(*t)));
                black_box(self.rot[i].sample(black_box(*t)));
            }
        }
    }
}

type BenchWide4 = Bench<TrackFixed<Vec3x4>, TrackFixed<Quatx4>>;

impl BenchWide4 {
    fn setup() -> Self {
        let mut rng = rand::thread_rng();

        let mut p: Vec<Vec3x4> = (0..16).map(|_| Vec3x4::zero()).collect();
        unsafe {
            rng.fill(std::slice::from_raw_parts_mut(
                p.as_mut_ptr() as *mut f32,
                p.len() * 3 * 4,
            ));
        }
        let pos = TrackFixed::from_keyframes(15, 0, p);

        let mut r: Vec<Quatx4> = (0..16)
            .map(|_| Quatx4(Vec4x4::new_splat(0.0, 0.0, 0.0, 1.0)))
            .collect();
        unsafe {
            rng.fill(std::slice::from_raw_parts_mut(
                r.as_mut_ptr() as *mut f32,
                r.len() * 4 * 4,
            ));
        }
        for x in &mut r {
            x.0.normalize();
        }
        let rot = TrackFixed::from_keyframes(15, 0, r);

        let mut t = [0.0; 64];
        rng.fill(&mut t);

        Self { t, pos, rot }
    }

    fn run(&self, iterations: usize) {
        for t in self.t.iter().cycle().take(iterations) {
            black_box(self.pos.sample(black_box(*t)));
            black_box(self.rot.sample(black_box(*t)));
        }
    }
}

type BenchWide8 = Bench<TrackFixed<Vec3x8>, TrackFixed<Quatx8>>;

impl BenchWide8 {
    fn setup() -> Self {
        let mut rng = rand::thread_rng();

        let mut p: Vec<Vec3x8> = (0..16).map(|_| Vec3x8::zero()).collect();
        unsafe {
            rng.fill(std::slice::from_raw_parts_mut(
                p.as_mut_ptr() as *mut f32,
                p.len() * 3 * 8,
            ));
        }
        let pos = TrackFixed::from_keyframes(15, 0, p);

        let mut r: Vec<Quatx8> = (0..16)
            .map(|_| Quatx8(Vec4x8::new_splat(0.0, 0.0, 0.0, 1.0)))
            .collect();
        unsafe {
            rng.fill(std::slice::from_raw_parts_mut(
                r.as_mut_ptr() as *mut f32,
                r.len() * 4 * 8,
            ));
        }
        for x in &mut r {
            x.0.normalize();
        }
        let rot = TrackFixed::from_keyframes(15, 0, r);

        let mut t = [0.0; 64];
        rng.fill(&mut t);

        Self { t, pos, rot }
    }

    fn run(&self, iterations: usize) {
        for t in self.t.iter().cycle().take(iterations) {
            black_box(self.pos.sample(black_box(*t)));
            black_box(self.rot.sample(black_box(*t)));
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
