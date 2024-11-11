use std::time::Duration;

use bevy_math::{prelude::*, FromRng, ShapeSample};
use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

const SAMPLES: usize = 100_000;

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
                black_box(shape.local_ray_cast(*ray, f32::MAX, false));
            });
        });
    });
}

fn ray_cast_3d(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_cast_3d_100k");
    group.warm_up_time(Duration::from_millis(500));

    let mut rng = StdRng::seed_from_u64(46);

    bench_shape(&mut group, &mut rng, "sphere", |rng| {
        Sphere::new(rng.gen_range(0.1..2.5))
    });
    bench_shape(&mut group, &mut rng, "cuboid", |rng| {
        Cuboid::new(
            rng.gen_range(0.1..5.0),
            rng.gen_range(0.1..5.0),
            rng.gen_range(0.1..5.0),
        )
    });
    bench_shape(&mut group, &mut rng, "cylinder", |rng| {
        Cylinder::new(rng.gen_range(0.1..2.5), rng.gen_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "cone", |rng| {
        Cone::new(rng.gen_range(0.1..2.5), rng.gen_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "conical_frustum", |rng| {
        ConicalFrustum {
            radius_top: rng.gen_range(0.1..2.5),
            radius_bottom: rng.gen_range(0.1..2.5),
            height: rng.gen_range(0.1..5.0),
        }
    });
    bench_shape(&mut group, &mut rng, "capsule3d", |rng| {
        Capsule3d::new(rng.gen_range(0.1..2.5), rng.gen_range(0.1..5.0))
    });
    bench_shape(&mut group, &mut rng, "triangle3d", |rng| {
        Triangle3d::new(
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
        )
    });
    bench_shape(&mut group, &mut rng, "tetrahedron", |rng| {
        Tetrahedron::new(
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
            Vec3::new(
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
                rng.gen_range(-7.5..7.5),
            ),
        )
    });
    bench_shape(&mut group, &mut rng, "torus", |rng| {
        Torus::new(rng.gen_range(0.1..1.25), rng.gen_range(1.26..2.5))
    });
}

criterion_group!(benches, ray_cast_3d);
criterion_main!(benches);
