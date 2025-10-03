use benches::bench;
use bevy_math::{
    bounding::{Aabb3d, BoundingSphere, BoundingVolume},
    prelude::*,
};
use core::hint::black_box;
use criterion::{criterion_group, Criterion};
use rand::{
    distr::{Distribution, StandardUniform, Uniform},
    rngs::StdRng,
    Rng, SeedableRng,
};

criterion_group!(benches, bounding);

struct PointCloud {
    points: Vec<Vec3A>,
    isometry: Isometry3d,
}

impl PointCloud {
    fn aabb(&self) -> Aabb3d {
        Aabb3d::from_point_cloud(self.isometry, self.points.iter().copied())
    }

    fn sphere(&self) -> BoundingSphere {
        BoundingSphere::from_point_cloud(self.isometry, &self.points)
    }
}

fn bounding(c: &mut Criterion) {
    // Create point clouds of various sizes, then benchmark two different ways
    // of finding the bounds of each cloud and merging them together.

    let mut rng1 = StdRng::seed_from_u64(123);
    let mut rng2 = StdRng::seed_from_u64(456);

    let point_clouds = Uniform::<usize>::new(black_box(3), black_box(30))
        .unwrap()
        .sample_iter(&mut rng1)
        .take(black_box(1000))
        .map(|num_points| PointCloud {
            points: StandardUniform
                .sample_iter(&mut rng2)
                .take(num_points)
                .collect::<Vec<Vec3A>>(),
            isometry: Isometry3d::new(rng2.random::<Vec3>(), rng2.random::<Quat>()),
        })
        .collect::<Vec<_>>();

    c.bench_function(bench!("bounding"), |b| {
        b.iter(|| {
            let aabb = point_clouds
                .iter()
                .map(PointCloud::aabb)
                .reduce(|l, r| l.merge(&r));

            let sphere = point_clouds
                .iter()
                .map(PointCloud::sphere)
                .reduce(|l, r| l.merge(&r));

            black_box(aabb);
            black_box(sphere);
        });
    });
}
