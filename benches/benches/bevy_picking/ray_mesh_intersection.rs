use core::hint::black_box;
use std::time::Duration;

use benches::bench;
use bevy_math::{Dir3, Mat4, Ray3d, Vec3};
use bevy_picking::mesh_picking::ray_cast;
use criterion::{criterion_group, BenchmarkId, Criterion};

criterion_group!(benches, bench);

fn ptoxznorm(p: u32, size: u32) -> (f32, f32) {
    let ij = (p / (size), p % (size));
    (ij.0 as f32 / size as f32, ij.1 as f32 / size as f32)
}

struct SimpleMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn mesh_creation(vertices_per_side: u32) -> SimpleMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for p in 0..vertices_per_side.pow(2) {
        let xz = ptoxznorm(p, vertices_per_side);
        positions.push([xz.0 - 0.5, 0.0, xz.1 - 0.5]);
        normals.push([0.0, 1.0, 0.0]);
    }

    let mut indices = vec![];
    for p in 0..vertices_per_side.pow(2) {
        if p % (vertices_per_side) != vertices_per_side - 1
            && p / (vertices_per_side) != vertices_per_side - 1
        {
            indices.extend_from_slice(&[p, p + 1, p + vertices_per_side]);
            indices.extend_from_slice(&[p + vertices_per_side, p + 1, p + vertices_per_side + 1]);
        }
    }

    SimpleMesh {
        positions,
        normals,
        indices,
    }
}

/// An enum that represents the configuration for all variations of the ray mesh intersection
/// benchmarks.
enum Benchmarks {
    Normal,
    NoCull,
    NoIntersection,
}

impl Benchmarks {
    const WARM_UP_TIME: Duration = Duration::from_millis(500);
    const VERTICES_PER_SIDE: [u32; 3] = [10, 100, 1000];

    /// Returns an iterator over every variant in this enum.
    fn iter() -> impl Iterator<Item = Self> {
        [Self::Normal, Self::NoCull, Self::NoIntersection].into_iter()
    }

    /// Returns the benchmark group name.
    fn name(&self) -> &'static str {
        match *self {
            Self::Normal => bench!("normal"),
            Self::NoCull => bench!("no_cull"),
            Self::NoIntersection => bench!("no_intersection"),
        }
    }

    fn ray(&self) -> Ray3d {
        Ray3d::new(
            Vec3::new(0.0, 1.0, 0.0),
            match *self {
                Self::Normal | Self::NoCull => Dir3::NEG_Y,
                // `NoIntersection` should not hit the mesh, so it goes an orthogonal direction.
                Self::NoIntersection => Dir3::X,
            },
        )
    }

    fn mesh_to_world(&self) -> Mat4 {
        Mat4::IDENTITY
    }

    fn backface_culling(&self) -> ray_cast::Backfaces {
        match *self {
            Self::Normal | Self::NoIntersection => ray_cast::Backfaces::Cull,
            Self::NoCull => ray_cast::Backfaces::Include,
        }
    }
}

fn bench(c: &mut Criterion) {
    for benchmark in Benchmarks::iter() {
        let mut group = c.benchmark_group(benchmark.name());

        group.warm_up_time(Benchmarks::WARM_UP_TIME);

        for vertices_per_side in Benchmarks::VERTICES_PER_SIDE {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}_vertices", vertices_per_side.pow(2))),
                &vertices_per_side,
                |b, &vertices_per_side| {
                    let ray = black_box(benchmark.ray());
                    let mesh_to_world = black_box(benchmark.mesh_to_world());
                    let mesh = black_box(mesh_creation(vertices_per_side));
                    let backface_culling = black_box(benchmark.backface_culling());

                    b.iter(|| {
                        ray_cast::ray_mesh_intersection(
                            ray,
                            &mesh_to_world,
                            &mesh.positions,
                            Some(&mesh.normals),
                            Some(&mesh.indices),
                            backface_culling,
                        )
                    });
                },
            );
        }
    }
}
