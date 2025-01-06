use core::hint::black_box;

use bevy_tasks::{ParallelIterator, TaskPoolBuilder};
use criterion::{criterion_group, BenchmarkId, Criterion};

struct ParChunks<'a, T>(core::slice::Chunks<'a, T>);
impl<'a, T> ParallelIterator<core::slice::Iter<'a, T>> for ParChunks<'a, T>
where
    T: 'a + Send + Sync,
{
    fn next_batch(&mut self) -> Option<core::slice::Iter<'a, T>> {
        self.0.next().map(|s| s.iter())
    }
}

struct ParChunksMut<'a, T>(core::slice::ChunksMut<'a, T>);
impl<'a, T> ParallelIterator<core::slice::IterMut<'a, T>> for ParChunksMut<'a, T>
where
    T: 'a + Send + Sync,
{
    fn next_batch(&mut self) -> Option<core::slice::IterMut<'a, T>> {
        self.0.next().map(|s| s.iter_mut())
    }
}

fn bench_overhead(c: &mut Criterion) {
    fn noop(_: &mut usize) {}

    let mut v = (0..10000).collect::<Vec<usize>>();
    c.bench_function("overhead_iter", |b| {
        b.iter(|| {
            v.iter_mut().for_each(noop);
        });
    });

    let mut v = (0..10000).collect::<Vec<usize>>();
    let mut group = c.benchmark_group("overhead_par_iter");
    for thread_count in &[1, 2, 4, 8, 16, 32] {
        let pool = TaskPoolBuilder::new().num_threads(*thread_count).build();
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            thread_count,
            |b, _| {
                b.iter(|| {
                    ParChunksMut(v.chunks_mut(100)).for_each(&pool, noop);
                });
            },
        );
    }
    group.finish();
}

fn bench_for_each(c: &mut Criterion) {
    fn busy_work(n: usize) {
        let mut i = n;
        while i > 0 {
            i = black_box(i - 1);
        }
    }

    let mut v = (0..10000).collect::<Vec<usize>>();
    c.bench_function("for_each_iter", |b| {
        b.iter(|| {
            v.iter_mut().for_each(|x| {
                busy_work(10000);
                *x = x.wrapping_mul(*x);
            });
        });
    });

    let mut v = (0..10000).collect::<Vec<usize>>();
    let mut group = c.benchmark_group("for_each_par_iter");
    for thread_count in &[1, 2, 4, 8, 16, 32] {
        let pool = TaskPoolBuilder::new().num_threads(*thread_count).build();
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            thread_count,
            |b, _| {
                b.iter(|| {
                    ParChunksMut(v.chunks_mut(100)).for_each(&pool, |x| {
                        busy_work(10000);
                        *x = x.wrapping_mul(*x);
                    });
                });
            },
        );
    }
    group.finish();
}

fn bench_many_maps(c: &mut Criterion) {
    fn busy_doubles(mut x: usize, n: usize) -> usize {
        for _ in 0..n {
            x = black_box(x.wrapping_mul(2));
        }
        x
    }

    let v = (0..10000).collect::<Vec<usize>>();
    c.bench_function("many_maps_iter", |b| {
        b.iter(|| {
            v.iter()
                .map(|x| busy_doubles(*x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .map(|x| busy_doubles(x, 1000))
                .for_each(drop);
        });
    });

    let v = (0..10000).collect::<Vec<usize>>();
    let mut group = c.benchmark_group("many_maps_par_iter");
    for thread_count in &[1, 2, 4, 8, 16, 32] {
        let pool = TaskPoolBuilder::new().num_threads(*thread_count).build();
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            thread_count,
            |b, _| {
                b.iter(|| {
                    ParChunks(v.chunks(100))
                        .map(|x| busy_doubles(*x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .map(|x| busy_doubles(x, 1000))
                        .for_each(&pool, drop);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_overhead, bench_for_each, bench_many_maps);
