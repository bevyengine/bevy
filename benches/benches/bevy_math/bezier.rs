use benches::bench;
use bevy_math::{prelude::*, VectorSpace};
use core::hint::black_box;
use criterion::{
    criterion_group, measurement::Measurement, BatchSize, BenchmarkGroup, BenchmarkId, Criterion,
};

criterion_group!(benches, segment_ease, curve_position, curve_iter_positions);

fn segment_ease(c: &mut Criterion) {
    let segment = black_box(CubicSegment::new_bezier_easing(
        vec2(0.25, 0.1),
        vec2(0.25, 1.0),
    ));

    c.bench_function(bench!("segment_ease"), |b| {
        let mut t = 0;

        b.iter_batched(
            || {
                // Increment `t` by 1, but use modulo to constrain it to `0..=1000`.
                t = (t + 1) % 1001;

                // Return time as a decimal between 0 and 1, inclusive.
                t as f32 / 1000.0
            },
            |t| segment.ease(t),
            BatchSize::SmallInput,
        );
    });
}

fn curve_position(c: &mut Criterion) {
    /// A helper function that benchmarks calling [`CubicCurve::position()`] over a generic [`VectorSpace`].
    fn bench_curve<M: Measurement, P: VectorSpace<Scalar = f32>>(
        group: &mut BenchmarkGroup<M>,
        name: &str,
        curve: CubicCurve<P>,
    ) {
        group.bench_with_input(BenchmarkId::from_parameter(name), &curve, |b, curve| {
            b.iter(|| curve.position(black_box(0.5)));
        });
    }

    let mut group = c.benchmark_group(bench!("curve_position"));

    let bezier_2 = CubicBezier::new([[
        vec2(0.0, 0.0),
        vec2(0.0, 1.0),
        vec2(1.0, 0.0),
        vec2(1.0, 1.0),
    ]])
    .to_curve()
    .unwrap();

    bench_curve(&mut group, "vec2", bezier_2);

    let bezier_3 = CubicBezier::new([[
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0),
        vec3(1.0, 1.0, 1.0),
    ]])
    .to_curve()
    .unwrap();

    bench_curve(&mut group, "vec3", bezier_3);

    let bezier_3a = CubicBezier::new([[
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 0.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]])
    .to_curve()
    .unwrap();

    bench_curve(&mut group, "vec3a", bezier_3a);

    group.finish();
}

fn curve_iter_positions(c: &mut Criterion) {
    let bezier = CubicBezier::new([[
        vec3a(0.0, 0.0, 0.0),
        vec3a(0.0, 1.0, 0.0),
        vec3a(1.0, 0.0, 0.0),
        vec3a(1.0, 1.0, 1.0),
    ]])
    .to_curve()
    .unwrap();

    c.bench_function(bench!("curve_iter_positions"), |b| {
        b.iter(|| {
            for x in bezier.iter_positions(black_box(100)) {
                // Discard `x`, since we just care about `iter_positions()` being consumed, but make
                // the compiler believe `x` is being used so it doesn't eliminate the iterator.
                black_box(x);
            }
        });
    });
}
