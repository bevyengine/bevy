use std::{fmt::Write, str, time::Duration};

use bevy_reflect::ParsedPath;
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use rand::{distributions::Uniform, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

criterion_group!(benches, parse_reflect_path);
criterion_main!(benches);

const WARM_UP_TIME: Duration = Duration::from_millis(500);
const MEASUREMENT_TIME: Duration = Duration::from_secs(2);
const SAMPLE_SIZE: usize = 500;
const NOISE_THRESHOLD: f64 = 0.03;
const SIZES: [usize; 6] = [100, 3160, 1000, 3_162, 10_000, 24_000];

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}
fn random_ident(rng: &mut ChaCha8Rng, f: &mut dyn Write) {
    let between = Uniform::try_from(b'a'..=b'z').unwrap();
    let ident_size = rng.gen_range(1..128);
    let ident: Vec<u8> = rng.sample_iter(between).take(ident_size).collect();
    let ident = str::from_utf8(&ident).unwrap();
    let _ = write!(f, "{ident}");
}

fn random_index(rng: &mut ChaCha8Rng, f: &mut dyn Write) {
    let index = rng.gen_range(1..128);
    let _ = write!(f, "{index}");
}

fn write_random_access(rng: &mut ChaCha8Rng, f: &mut dyn Write) {
    match rng.gen_range(0..4) {
        0 => {
            // Access::Field
            f.write_char('.').unwrap();
            random_ident(rng, f);
        }
        1 => {
            // Access::FieldIndex
            f.write_char('#').unwrap();
            random_index(rng, f);
        }
        2 => {
            // Access::Index
            f.write_char('[').unwrap();
            random_index(rng, f);
            f.write_char(']').unwrap();
        }
        3 => {
            // Access::TupleIndex
            f.write_char('.').unwrap();
            random_index(rng, f);
        }
        _ => unreachable!(),
    }
}

fn mk_paths(size: usize) -> impl FnMut() -> String {
    let mut rng = deterministic_rand();
    move || {
        let mut ret = String::new();
        (0..size).for_each(|_| write_random_access(&mut rng, &mut ret));
        ret
    }
}

fn parse_reflect_path(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("parse_reflect_path");
    group.warm_up_time(WARM_UP_TIME);
    group.measurement_time(MEASUREMENT_TIME);
    group.sample_size(SAMPLE_SIZE);
    group.noise_threshold(NOISE_THRESHOLD);
    let group = &mut group;

    for size in SIZES {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_reflect_path", size),
            &size,
            |bencher, &size| {
                let mut mk_paths = mk_paths(size);
                bencher.iter_batched(
                    || mk_paths(),
                    |path| assert!(ParsedPath::parse(black_box(&path)).is_ok()),
                    BatchSize::SmallInput,
                );
            },
        );
    }
}
