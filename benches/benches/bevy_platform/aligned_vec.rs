use benches::bench;
use bevy_math::prelude::*;
use bevy_platform::collections::AlignedVec;
use bytemuck::{Pod, Zeroable};
use chacha20::ChaCha8Rng;
use core::hint::black_box;
use criterion::{
    criterion_group, measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion,
};
use rand::{RngExt, SeedableRng};

criterion_group!(benches, bytes_read_write);

#[derive(Debug, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct S64A16 {
    mat: [Vec4; 4],
}

#[derive(Debug, Pod, Zeroable, Clone, Copy)]
#[repr(C, align(128))]
struct S128A128 {
    c1: Vec4,        // 16, 16
    c2: Vec4,        // 16, 32
    pad1: [u32; 8],  // 32, 64
    pad2: [u32; 16], // 64, 128
}

const ELEMENT_COUNTS: &[usize] = &[100000];

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

fn bytes_read_write(c: &mut Criterion) {
    bench_group::<u8, 1, _>(&mut c.benchmark_group(bench!("size1_align1")));
    bench_group::<S64A16, 64, _>(&mut c.benchmark_group(bench!("size64_align16")));
    bench_group::<S128A128, 128, _>(&mut c.benchmark_group(bench!("size128_align128")));

    fn bench_group<T: Pod, const S: usize, M: Measurement>(group: &mut BenchmarkGroup<'_, M>) {
        const {
            assert!(size_of::<T>() == S);
        }
        for element_count in ELEMENT_COUNTS {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("read_{}_elements_unaligned", element_count)),
                element_count,
                |b, &element_count| {
                    let bytes = create_bytes_unaligned::<T>(element_count);

                    b.iter(|| {
                        let mut rng = deterministic_rand();
                        for _ in 0..element_count {
                            let i = rng.random_range(0..element_count);
                            let t: T = bytemuck::pod_read_unaligned(
                                &bytes[(i * size_of::<T>())..((i + 1) * size_of::<T>())],
                            );
                            black_box(t);
                        }
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("read_{}_elements_aligned", element_count)),
                element_count,
                |b, &element_count| {
                    let bytes = create_bytes_aligned::<T, S>(element_count);
                    let slice = bytes.cast_slice::<T>();

                    b.iter(|| {
                        let mut rng = deterministic_rand();
                        for _ in 0..element_count {
                            let i = rng.random_range(0..element_count);
                            let t: T = slice[i];
                            black_box(t);
                        }
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("write_{}_elements_unaligned", element_count)),
                element_count,
                |b, &element_count| {
                    let mut bytes = create_bytes_unaligned::<T>(element_count);

                    b.iter(|| {
                        let mut rng = deterministic_rand();
                        for _ in 0..element_count {
                            let i = rng.random_range(0..element_count);
                            let write_idx = element_count - 1 - i;
                            bytes[(write_idx * size_of::<T>())..((write_idx + 1) * size_of::<T>())]
                                .copy_from_slice(&create_seq_array::<T, S>(i));
                        }
                    });
                },
            );
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("write_{}_elements_aligned", element_count)),
                element_count,
                |b, &element_count| {
                    let mut bytes = create_bytes_aligned::<T, S>(element_count);
                    let slice = bytes.cast_slice_mut::<T>();

                    b.iter(|| {
                        let mut rng = deterministic_rand();
                        for _ in 0..element_count {
                            let i = rng.random_range(0..element_count);
                            let write_idx = element_count - 1 - i;
                            slice[write_idx] = bytemuck::must_cast(create_seq_array::<T, S>(i));
                        }
                    });
                },
            );
        }
    }
}

fn create_bytes_unaligned<T>(element_count: usize) -> Vec<u8> {
    Vec::<u8>::from_iter((0..element_count * size_of::<T>()).map(|i| i as u8))
}

fn create_bytes_aligned<T: Pod, const S: usize>(element_count: usize) -> AlignedVec {
    AlignedVec::from(Vec::<T>::from_iter(
        (0..element_count).map(|i| bytemuck::must_cast(create_seq_array::<T, S>(i))),
    ))
}

fn create_seq_array<T, const S: usize>(i: usize) -> [u8; S] {
    core::array::from_fn::<u8, S, _>(|ofs| (i * size_of::<T>() + ofs) as u8)
}
