use bevy::math::{
    curves::{Curve, KeyframeIndex, CurveVariable},
    Vec4,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;

criterion_group!(benches, curve_variable);
criterion_main!(benches);

const SAMPLES_COUNT: usize = 100;

fn curve_sampling<T>(samples: &[f32], curve: &impl Curve<Output = T>) {
    let mut c: KeyframeIndex = 0;
    for t in samples {
        let (nc, v) = curve.sample_with_cursor(c, *t);
        black_box(v);
        c = nc;
    }
}

fn curve_variable(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("curve_variable");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));

    let curve = CurveVariable::with_auto_tangents(
        vec![0.0, 1.0, 1.3, 1.6, 1.7, 1.8, 1.9, 2.0],
        vec![3.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25, 0.0]
            .iter()
            .map(|x| Vec4::splat(*x))
            .collect::<Vec<_>>(),
    )
    .unwrap();

    let duration = curve.duration();
    let mut rand = rand::thread_rng();

    let rand_samples = (0..SAMPLES_COUNT)
        .into_iter()
        .map(|_| duration * rand.gen::<f32>())
        .collect::<Vec<_>>();

    let samples = (0..SAMPLES_COUNT)
        .into_iter()
        .map(|i| duration * (i as f32) / (SAMPLES_COUNT - 1) as f32)
        .collect::<Vec<_>>();

    group.bench_function("random_sampling", |bencher| {
        bencher.iter(|| black_box(curve_sampling(&rand_samples[..], &curve)));
    });

    group.bench_function("sampling", |bencher| {
        bencher.iter(|| black_box(curve_sampling(&samples[..], &curve)));
    });

    group.finish()
}
