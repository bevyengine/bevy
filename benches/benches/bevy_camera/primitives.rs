use bevy_camera::primitives::{Aabb, Frustum, Sphere};
use bevy_math::{
    primitives::{HalfSpace, ViewFrustum},
    Affine3A, Quat, Vec3, Vec3A, Vec4,
};
use core::hint::black_box;
use criterion::{criterion_group, Criterion};

pub fn intersects_obb(c: &mut Criterion) {
    let mut group = c.benchmark_group("intersects_obb");

    let aabb = Aabb {
        center: Vec3A::ZERO,
        half_extents: Vec3A::new(0.5, 0.5, 0.5),
    };

    let world_from_local = Affine3A::from_rotation_translation(
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
        Vec3::new(1.0, 0.5, -0.5),
    );

    let identity_transform = Affine3A::IDENTITY;

    let sphere = Sphere {
        center: Vec3A::new(1.0, 0.5, 0.0),
        radius: 1.5,
    };

    let frustum = Frustum(ViewFrustum {
        half_spaces: [
            HalfSpace::new(Vec4::new(-0.9701, -0.2425, -0.0000, 0.7276)),
            HalfSpace::new(Vec4::new(-0.0000, 1.0000, -0.0000, 1.0000)),
            HalfSpace::new(Vec4::new(-0.0000, -0.2425, -0.9701, 0.7276)),
            HalfSpace::new(Vec4::new(-0.0000, -1.0000, -0.0000, 1.0000)),
            HalfSpace::new(Vec4::new(-0.0000, -0.2425, 0.9701, 0.7276)),
            HalfSpace::new(Vec4::new(0.9701, -0.2425, -0.0000, 0.7276)),
        ],
    });

    assert!(sphere.intersects_obb(&aabb, &world_from_local));
    group.bench_function("sphere_intersects_obb", |b| {
        b.iter(|| black_box(sphere.intersects_obb(black_box(&aabb), black_box(&world_from_local))));
    });

    assert!(frustum.intersects_obb(&aabb, &world_from_local, true, true));
    group.bench_function("frustum_intersects_obb", |b| {
        b.iter(|| {
            black_box(frustum.intersects_obb(
                black_box(&aabb),
                black_box(&world_from_local),
                black_box(true), // intersect_near
                black_box(true), // intersect_far
            ))
        });
    });

    assert!(frustum.intersects_obb(&aabb, &identity_transform, true, true));
    group.bench_function("frustum_intersects_obb_fallback_identity", |b| {
        b.iter(|| {
            black_box(frustum.intersects_obb(
                black_box(&aabb),
                black_box(&identity_transform),
                black_box(true),
                black_box(true),
            ))
        });
    });

    assert!(frustum.intersects_obb_identity(&aabb));
    group.bench_function("frustum_intersects_obb_identity", |b| {
        b.iter(|| black_box(frustum.intersects_obb_identity(black_box(&aabb))));
    });

    group.finish();
}

criterion_group!(benches, intersects_obb);
