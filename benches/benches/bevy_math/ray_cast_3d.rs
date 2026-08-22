use std::time::Duration;

use bevy_math::{prelude::*, FromRng, ShapeSample};
use criterion::{criterion_group, measurement::WallTime, BenchmarkGroup, Criterion};
use rand::{rngs::StdRng, RngExt as _, SeedableRng};

const SAMPLES: usize = 100_000;

criterion_group!(benches, ray_cast_3d);

fn bench_shape<S: PrimitiveRayCast3d>(
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
            .map(|_| Ray3d {
                origin: Sphere::new(10.0).sample_interior(rng),
                direction: Dir3::from_rng(rng),
            })
            .collect::<Vec<_>>();
        let items = shapes.into_iter().zip(rays).collect::<Vec<_>>();

        // Cast rays against the shapes.
        b.iter(|| {
            items.iter().for_each(|(shape, ray)| {
                core::hint::black_box(shape.local_ray_cast(*ray, f32::MAX, false));
            });
        });
    });
}

fn ray_cast_3d(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_cast_3d_100k");
    group.warm_up_time(Duration::from_millis(500));

    let mut rng = StdRng::seed_from_u64(46);

    bench_shape(&mut group, &mut rng, "sphere", |rng| {
        Sphere::new(rng.random_range(0.1..2.5))
    });
    bench_shape(&mut group, &mut rng, "cuboid", |rng| {
        Cuboid::new(
            rng.random_range(0.1..5.0),
            rng.random_range(0.1..5.0),
            rng.random_range(0.1..5.0),
        )
    });
    bench_shape(&mut group, &mut rng, "cylinder", |rng| {
        Cylinder::new(rng.random_range(0.1..2.5), rng.random_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "cone", |rng| {
        Cone::new(rng.random_range(0.1..2.5), rng.random_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "conical_frustum", |rng| {
        ConicalFrustum {
            radius_top: rng.random_range(0.1..2.5),
            radius_bottom: rng.random_range(0.1..2.5),
            height: rng.random_range(0.1..5.0),
        }
    });
    bench_shape(&mut group, &mut rng, "capsule3d", |rng| {
        Capsule3d::new(rng.random_range(0.1..2.5), rng.random_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "triangle3d", |rng| {
        Triangle3d::new(
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
        )
    });
    bench_shape(&mut group, &mut rng, "tetrahedron", |rng| {
        Tetrahedron::new(
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
                rng.random_range(-7.5..7.5),
            ),
        )
    });
    bench_shape(&mut group, &mut rng, "torus", |rng| {
        Torus::new(rng.random_range(0.1..1.25), rng.random_range(1.26..2.5))
    });
}
