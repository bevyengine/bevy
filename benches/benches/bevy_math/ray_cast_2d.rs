use std::time::Duration;

use bevy_math::{prelude::*, FromRng, ShapeSample};
use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

const SAMPLES: usize = 100_000;

fn bench_shape<S: PrimitiveRayCast2d>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    rng: &mut StdRng,
    name: &str,
    shape_constructor: impl Fn(&mut StdRng) -> S,
) {
    group.bench_function(format!("{name}_ray_cast"), |b| {
        // Generate random shapes and rays.
        let shapes = (0..SAMPLES)
            .map(|_| shape_constructor(rng))
            .collect::<Vec<_>>();
        let rays = (0..SAMPLES)
            .map(|_| Ray2d {
                origin: Circle::new(10.0).sample_interior(rng),
                direction: Dir2::from_rng(rng),
            })
            .collect::<Vec<_>>();
        let items = shapes.into_iter().zip(rays).collect::<Vec<_>>();

        // Cast rays against the shapes.
        b.iter(|| {
            items.iter().for_each(|(shape, ray)| {
                black_box(shape.local_ray_cast(*ray, f32::MAX, false));
            });
        });
    });
}

fn ray_cast_2d(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_cast_2d_100k");
    group.warm_up_time(Duration::from_millis(500));

    let mut rng = StdRng::seed_from_u64(46);

    bench_shape(&mut group, &mut rng, "circle", |rng| {
        Circle::new(rng.gen_range(0.1..2.5))
    });
    bench_shape(&mut group, &mut rng, "arc", |rng| {
        Arc2d::new(
            rng.gen_range(0.1..2.5),
            rng.gen_range(0.1..std::f32::consts::PI),
        )
    });
    bench_shape(&mut group, &mut rng, "circular_sector", |rng| {
        CircularSector::new(
            rng.gen_range(0.1..2.5),
            rng.gen_range(0.1..std::f32::consts::PI),
        )
    });
    bench_shape(&mut group, &mut rng, "circular_segment", |rng| {
        CircularSegment::new(
            rng.gen_range(0.1..2.5),
            rng.gen_range(0.1..std::f32::consts::PI),
        )
    });
    bench_shape(&mut group, &mut rng, "ellipse", |rng| {
        Ellipse::new(rng.gen_range(0.1..2.5), rng.gen_range(0.1..2.5))
    });
    bench_shape(&mut group, &mut rng, "annulus", |rng| {
        Annulus::new(rng.gen_range(0.1..1.25), rng.gen_range(1.26..2.5))
    });
    bench_shape(&mut group, &mut rng, "capsule2d", |rng| {
        Capsule2d::new(rng.gen_range(0.1..1.25), rng.gen_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "rectangle", |rng| {
        Rectangle::new(rng.gen_range(0.1..5.0), rng.gen_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "rhombus", |rng| {
        Rhombus::new(rng.gen_range(0.1..5.0), rng.gen_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "line2d", |rng| Line2d {
        direction: Dir2::from_rng(rng),
    });
    bench_shape(&mut group, &mut rng, "segment2d", |rng| Segment2d {
        direction: Dir2::from_rng(rng),
        half_length: rng.gen_range(0.1..5.0),
    });
    bench_shape(&mut group, &mut rng, "regular_polygon", |rng| {
        RegularPolygon::new(rng.gen_range(0.1..2.5), rng.gen_range(3..6))
    });
    bench_shape(&mut group, &mut rng, "triangle2d", |rng| {
        Triangle2d::new(
            Vec2::new(rng.gen_range(-7.5..7.5), rng.gen_range(-7.5..7.5)),
            Vec2::new(rng.gen_range(-7.5..7.5), rng.gen_range(-7.5..7.5)),
            Vec2::new(rng.gen_range(-7.5..7.5), rng.gen_range(-7.5..7.5)),
        )
    });
}

criterion_group!(benches, ray_cast_2d);
criterion_main!(benches);
