use core::hint::black_box;
use std::sync::Arc;

use bevy_render::{
    impl_atomic_pod,
    render_resource::{AtomicPod, AtomicSparseBufferVec, BufferUsages, SparseBufferVec},
};
use bytemuck::{Pod, Zeroable};
use criterion::{criterion_group, Criterion, Throughput};
use rand::{rngs::StdRng, RngExt, SeedableRng};

/// A 16-byte POD element, similar in size to the data types these buffers
/// store in practice (e.g. mesh culling data).
#[derive(Clone, Copy, Pod, Zeroable, Default)]
#[repr(C)]
struct Element([f32; 4]);

impl_atomic_pod!(Element, ElementBlob);

/// The number of elements each benchmark buffer holds.
const ELEMENT_COUNT: usize = 65_536;
/// The number of random element updates performed per iteration of `set`.
const UPDATE_COUNT: usize = 1_024;

fn new_atomic() -> AtomicSparseBufferVec<Element> {
    AtomicSparseBufferVec::new(BufferUsages::STORAGE, Arc::from("sparse buffer bench"))
}

fn new_non_atomic() -> SparseBufferVec<Element> {
    SparseBufferVec::new(BufferUsages::STORAGE, Arc::from("sparse buffer bench"))
}

fn populate(buffer: &mut AtomicSparseBufferVec<Element>) {
    for i in 0..ELEMENT_COUNT {
        buffer.push(Element([i as f32, 0.0, 0.0, 0.0]));
    }
}

fn populate_non_atomic(buffer: &mut SparseBufferVec<Element>) {
    for i in 0..ELEMENT_COUNT {
        buffer.push(Element([i as f32, 0.0, 0.0, 0.0]));
    }
}

/// Compares appending new elements, which also grows the internal dirty-bit
/// bookkeeping.
fn push(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_buffer/push");
    group.throughput(Throughput::Elements(ELEMENT_COUNT as u64));

    group.bench_function("atomic", |b| {
        b.iter(|| {
            let mut buffer = new_atomic();
            for i in 0..ELEMENT_COUNT {
                buffer.push(Element([i as f32, 0.0, 0.0, 0.0]));
            }
            black_box(buffer.len());
        });
    });

    group.bench_function("non_atomic", |b| {
        b.iter(|| {
            let mut buffer = new_non_atomic();
            for i in 0..ELEMENT_COUNT {
                buffer.push(Element([i as f32, 0.0, 0.0, 0.0]));
            }
            black_box(buffer.len());
        });
    });

    group.finish();
}

/// Compares overwriting existing elements at random indices, which is the hot
/// path: each write also marks the element dirty.
fn set(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_buffer/set");
    group.throughput(Throughput::Elements(UPDATE_COUNT as u64));

    // Use a fixed seed so the set of updated indices is reproducible across
    // benchmark runs.
    let mut rng = StdRng::seed_from_u64(123);
    let indices: Vec<u32> = (0..UPDATE_COUNT)
        .map(|_| rng.random_range(0..ELEMENT_COUNT as u32))
        .collect();
    let values: Vec<Element> = (0..UPDATE_COUNT)
        .map(|i| Element([i as f32, 0.0, 0.0, 0.0]))
        .collect();

    let mut atomic = new_atomic();
    populate(&mut atomic);
    group.bench_function("set", |b| {
        b.iter(|| {
            for (&index, &value) in indices.iter().zip(values.iter()) {
                atomic.set(index, value);
            }
            black_box(atomic.len());
        });
    });

    group.bench_function("set_mut", |b| {
        b.iter(|| {
            for (&index, &value) in indices.iter().zip(values.iter()) {
                atomic.set_mut(index, value);
            }
            black_box(atomic.len());
        });
    });

    let mut non_atomic = new_non_atomic();
    populate_non_atomic(&mut non_atomic);
    group.bench_function("non_atomic_set", |b| {
        b.iter(|| {
            for (&index, &value) in indices.iter().zip(values.iter()) {
                non_atomic.set(index, value);
            }
            black_box(non_atomic.len());
        });
    });

    group.finish();
}

/// Compares reading elements back out of the buffer.
fn get(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_buffer/get");
    group.throughput(Throughput::Elements(ELEMENT_COUNT as u64));

    let mut atomic = new_atomic();
    populate(&mut atomic);
    group.bench_function("atomic", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..ELEMENT_COUNT {
                sum += atomic.get(i as u32).0[0];
            }
            black_box(sum);
        });
    });

    let mut non_atomic = new_non_atomic();
    populate_non_atomic(&mut non_atomic);
    group.bench_function("non_atomic", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..ELEMENT_COUNT {
                sum += non_atomic.get(i as u32).0[0];
            }
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(benches, push, set, get);
