use benches::bench;
use bevy_math::prelude::*;
use core::hint::black_box;
use criterion::{criterion_group, Criterion};

criterion_group!(benches, smoothstep);

#[inline(always)]
fn smoothstep_inline(t: f32) -> f32 {
    ((3.0 - 2.0 * t) * t) * t
}

#[inline(never)]
fn smoothstep_noinline(t: f32) -> f32 {
    smoothstep_inline(t)
}

fn smoothstep(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("smoothstep"));

    // First baseline - what if the function is fully inlined?
    group.bench_function("inline", |b| b.iter(|| smoothstep_inline(black_box(0.5))));

    // Second baseline - non-inlined.
    group.bench_function("noinline", |b| {
        b.iter(|| smoothstep_noinline(black_box(0.5)))
    });

    // EaseFunction interface.
    //
    // This should be a bit slower than `noinline` - the compiler doesn't like
    // to inline EaseFunction::eval so there's an extra branch.
    group.bench_function("function", |b| {
        b.iter(|| EaseFunction::SmoothStep.sample_unchecked(black_box(0.5)))
    });

    // EasingCurve interface with a constant unit range.
    //
    // Despite giving the same result as EaseFunction, this can be a bit slower
    // as the compiler can't completely eliminate the range remapping.
    // Eliminating it would require `lerp(0.0, 1.0, t)` to be equivalent to `t`,
    // which is not the case if `t` is `-0.0`.
    group.bench_function("curve", |b| {
        b.iter(|| {
            EasingCurve::new(0.0f32, 1.0f32, EaseFunction::SmoothStep)
                .sample_unchecked(black_box(0.5))
        })
    });
}
