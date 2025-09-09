use core::hint::black_box;
use std::time::Duration;

use benches::bench;
use bevy_math::{Affine3A, Dir3, Ray3d, Vec3};
use bevy_picking::mesh_picking::ray_cast::{self, Backfaces};
use criterion::{criterion_group, AxisScale, BenchmarkId, Criterion, PlotConfiguration};

criterion_group!(benches, bench);

/// A mesh that can be passed to [`ray_cast::ray_mesh_intersection()`].
struct SimpleMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

/// Selects a point within a normal square.
///
/// `p` is an index within `0..vertices_per_side.pow(2)`. The returned value is a coordinate where
/// both `x` and `z` are within `0..1`.
fn p_to_xz_norm(p: u32, vertices_per_side: u32) -> (f32, f32) {
    let x = (p / vertices_per_side) as f32;
    let z = (p % vertices_per_side) as f32;

    let vertices_per_side = vertices_per_side as f32;

    // Scale `x` and `z` to be between 0 and 1.
    (x / vertices_per_side, z / vertices_per_side)
}

fn create_mesh(vertices_per_side: u32) -> SimpleMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    for p in 0..vertices_per_side.pow(2) {
        let (x, z) = p_to_xz_norm(p, vertices_per_side);

        // Push a new vertex to the mesh. We translate all vertices so the final square is
        // centered at (0, 0), instead of (0.5, 0.5).
        positions.push([x - 0.5, 0.0, z - 0.5]);

        // All vertices have the same normal.
        normals.push([0.0, 1.0, 0.0]);

        // Extend the indices for for all vertices except for the final row and column, since
        // indices are "between" points.
        if p % vertices_per_side != vertices_per_side - 1
            && p / vertices_per_side != vertices_per_side - 1
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
    /// The ray intersects the mesh, and culling is enabled.
    CullHit,

    /// The ray intersects the mesh, and culling is disabled.
    NoCullHit,

    /// The ray does not intersect the mesh, and culling is enabled.
    CullMiss,
}

impl Benchmarks {
    const WARM_UP_TIME: Duration = Duration::from_millis(500);
    const VERTICES_PER_SIDE: [u32; 3] = [10, 100, 1000];

    /// Returns an iterator over every variant in this enum.
    fn iter() -> impl Iterator<Item = Self> {
        [Self::CullHit, Self::NoCullHit, Self::CullMiss].into_iter()
    }

    /// Returns the benchmark group name.
    fn name(&self) -> &'static str {
        match *self {
            Self::CullHit => bench!("cull_intersect"),
            Self::NoCullHit => bench!("no_cull_intersect"),
            Self::CullMiss => bench!("cull_no_intersect"),
        }
    }

    fn ray(&self) -> Ray3d {
        Ray3d::new(
            Vec3::new(0.0, 1.0, 0.0),
            match *self {
                Self::CullHit | Self::NoCullHit => Dir3::NEG_Y,
                // `NoIntersection` should not hit the mesh, so it goes an orthogonal direction.
                Self::CullMiss => Dir3::X,
            },
        )
    }

    fn mesh_to_world(&self) -> Affine3A {
        Affine3A::IDENTITY
    }

    fn backface_culling(&self) -> Backfaces {
        match *self {
            Self::CullHit | Self::CullMiss => Backfaces::Cull,
            Self::NoCullHit => Backfaces::Include,
        }
    }

    /// Returns whether the ray should intersect with the mesh.
    #[cfg(test)]
    fn should_intersect(&self) -> bool {
        match *self {
            Self::CullHit | Self::NoCullHit => true,
            Self::CullMiss => false,
        }
    }
}

/// A benchmark that times [`ray_cast::ray_mesh_intersection()`].
///
/// There are multiple different scenarios that are tracked, which are described by the
/// [`Benchmarks`] enum. Each scenario has its own benchmark group, where individual benchmarks
/// track a ray intersecting a square mesh of an increasing amount of vertices.
fn bench(c: &mut Criterion) {
    for benchmark in Benchmarks::iter() {
        let mut group = c.benchmark_group(benchmark.name());

        group
            .warm_up_time(Benchmarks::WARM_UP_TIME)
            // Make the scale logarithmic, to match `VERTICES_PER_SIDE`.
            .plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

        for vertices_per_side in Benchmarks::VERTICES_PER_SIDE {
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}_vertices", vertices_per_side.pow(2))),
                &vertices_per_side,
                |b, &vertices_per_side| {
                    let ray = black_box(benchmark.ray());
                    let mesh_to_world = black_box(benchmark.mesh_to_world());
                    let mesh = black_box(create_mesh(vertices_per_side));
                    let backface_culling = black_box(benchmark.backface_culling());

                    b.iter(|| {
                        let intersected = ray_cast::ray_mesh_intersection(
                            ray,
                            &mesh_to_world,
                            &mesh.positions,
                            Some(&mesh.normals),
                            Some(&mesh.indices),
                            None,
                            backface_culling,
                        );

                        #[cfg(test)]
                        assert_eq!(intersected.is_some(), benchmark.should_intersect());

                        intersected
                    });
                },
            );
        }
    }
}
