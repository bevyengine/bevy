use core::hint::black_box;
use std::time::Duration;

use benches::bench;
use bevy_math::{Dir3, Mat4, Ray3d, Vec2, Vec3};
use bevy_picking::mesh_picking::ray_cast::{self, Backfaces};
use bevy_render::mesh::{Indices, Mesh, MeshBuilder, PlaneMeshBuilder, VertexAttributeValues};
use criterion::{
    criterion_group, measurement::WallTime, AxisScale, BenchmarkGroup, BenchmarkId, Criterion,
    PlotConfiguration, Throughput,
};

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

        // Push a new vertice to the mesh. We translate all vertices so the final square is
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

fn cull_intersect(mut group: BenchmarkGroup<'_, WallTime>, should_intersect: bool) {
    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = create_mesh(vertices_per_side);
        let tri_count = (mesh.indices.len() / 3) as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
                underscore_separate_number(mesh.indices.len())
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    let intersection = ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Some(&mesh.indices),
                        Backfaces::Cull,
                    );

                    #[cfg(test)]
                    assert_eq!(intersection.is_some(), should_intersect);

                    intersection
                });
            },
        );
    }
}

fn no_cull_intersect(mut group: BenchmarkGroup<'_, WallTime>, should_intersect: bool) {
    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = create_mesh(vertices_per_side);
        let tri_count = (mesh.indices.len() / 3) as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
                underscore_separate_number(mesh.indices.len())
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    let intersection = ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Some(&mesh.indices),
                        Backfaces::Include,
                    );

                    #[cfg(test)]
                    assert_eq!(intersection.is_some(), should_intersect);

                    intersection
                });
            },
        );
    }
}

fn cull_no_intersect(mut group: BenchmarkGroup<'_, WallTime>, should_intersect: bool) {
    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::X);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = create_mesh(vertices_per_side);
        let tri_count = (mesh.indices.len() / 3) as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
                underscore_separate_number(mesh.indices.len())
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    let intersection = ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Some(&mesh.indices),
                        Backfaces::Cull,
                    );

                    #[cfg(test)]
                    assert_eq!(intersection.is_some(), should_intersect);

                    intersection
                });
            },
        );
    }
}

enum IntersectIndices {
    Include,
    Skip,
}

fn single_plane(
    mut group: BenchmarkGroup<'_, WallTime>,
    should_intersect: bool,
    use_indices: IntersectIndices,
) {
    for subdivisions in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.01, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mut mesh = PlaneMeshBuilder::new(Dir3::Y, Vec2::ONE)
            .subdivisions(subdivisions)
            .build();

        if matches!(use_indices, IntersectIndices::Skip) {
            // duplicate_mesh consumes the indices
            mesh.duplicate_vertices();
        }

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();

        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();

        let indices = mesh.indices();

        let tri_count = indices
            .map(|i| i.len() as u64 / 3)
            .unwrap_or(positions.len() as u64 / 3);

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(positions.len()),
                underscore_separate_number(indices.map(Indices::len).unwrap_or_default())
            ),
            &(ray, mesh_to_world, positions, normals, indices),
            |b, (ray, mesh_to_world, positions, normals, indices)| {
                b.iter(|| {
                    let intersection = ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        positions,
                        Some(normals),
                        match indices {
                            Some(Indices::U32(indices)) => Some(indices),
                            _ => None,
                        },
                        Backfaces::Cull,
                    );

                    #[cfg(test)]
                    assert_eq!(intersection.is_some(), should_intersect);

                    intersection
                });
            },
        );
    }
}

enum OverlappingPlaneOrdering {
    BackToFront,
    FrontToBack,
}

fn overlapping_planes(
    mut group: BenchmarkGroup<'_, WallTime>,
    should_intersect: bool,
    plane_ordering: OverlappingPlaneOrdering,
) {
    let pos_mul = match plane_ordering {
        OverlappingPlaneOrdering::BackToFront => 1.0,
        OverlappingPlaneOrdering::FrontToBack => -1.0,
    };

    for planes in [10_u32, 1000, 100_000] {
        let ray = Ray3d::new(Vec3::new(0.1, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;

        // let mut mesh = PlaneMeshBuilder::new(Dir3::Y, Vec2::ONE).build();

        // let copy_mesh = mesh.clone();

        // Generate a mesh of many planes with subsequent plane being further from ray origin
        // or closer depending on plane_ordering
        let mesh = (0..planes)
            .map(|i| {
                let mut plane_mesh = PlaneMeshBuilder::new(Dir3::Y, Vec2::ONE).build();

                if let VertexAttributeValues::Float32x3(positions) =
                    plane_mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap()
                {
                    positions.iter_mut().for_each(|vert| {
                        let plane_count = planes as f32;
                        let plane_height = -plane_count + pos_mul * i as f32;

                        vert[1] = plane_height;
                    });

                    // positions.iter_mut().enumerate().for_each(|(i, pos)| {
                    //     pos[1] -= (planes as usize + planes as usize * pos_mul - i / 4) as f32;
                    // });
                } else {
                    panic!("No positions");
                }

                plane_mesh
            })
            .reduce(|mut acc, next_mesh| {
                acc.merge(&next_mesh);
                acc
            })
            .unwrap();

        // for i in 1..(planes) {
        //     let mut next_plane = PlaneMeshBuilder::new(Dir3::Y, Vec2::ONE).build();

        //     if let VertexAttributeValues::Float32x3(positions) =
        //         next_plane.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap()
        //     {
        //         positions.iter_mut().for_each(|vert| {
        //             let plane_count = planes as f32;
        //             let plane_height = -plane_count + pos_mul * i as f32;

        //             vert[1] = plane_height;
        //         });

        //         // positions.iter_mut().enumerate().for_each(|(i, pos)| {
        //         //     pos[1] -= (planes as usize + planes as usize * pos_mul - i / 4) as f32;
        //         // });
        //     } else {
        //         panic!("No positions");
        //     }

        //     mesh.merge(&next_plane);
        // }

        // for _ in 0..(planes - 1) {
        //     mesh.merge(&copy_mesh);
        // }

        // if let VertexAttributeValues::Float32x3(positions) =
        //     mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap()
        // {
        //     println!("{}", positions.len());
        //     // place each subsequent plane closer to the ray origin

        //     const CHUNK_SIZE: usize = 4; // the number of vertices of a plane with 0 subdivisions
        //     positions
        //         .chunks_exact_mut(CHUNK_SIZE)
        //         .enumerate()
        //         // .take(1)
        //         .for_each(|(i, vertices)| {
        //             let plane_count = planes as f32;
        //             let plane_height = -plane_count + pos_mul * i as f32;
        //             // println!("{i} {plane_height}");

        //             // println!("{:?}", vertices);

        //             for v in vertices.iter_mut() {
        //                 v[1] = 0.1;
        //             }

        //             // println!("{:?}", vertices);
        //         });

        //     // positions.iter_mut().enumerate().for_each(|(i, pos)| {
        //     //     pos[1] -= (planes as usize + planes as usize * pos_mul - i / 4) as f32;
        //     // });
        // } else {
        //     panic!("No positions");
        // }

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();

        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();

        let indices = mesh.indices();

        let tri_count = indices
            .map(|i| i.len() as u64 / 3)
            .unwrap_or(positions.len() as u64 / 3);

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(positions.len()),
                underscore_separate_number(indices.unwrap().len())
            ),
            &(ray, mesh_to_world, positions, normals, indices),
            |b, (ray, mesh_to_world, positions, normals, indices)| {
                b.iter(|| {
                    let intersection = ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        positions,
                        Some(normals),
                        match indices {
                            Some(Indices::U32(indices)) => Some(indices),
                            _ => None,
                        },
                        Backfaces::Cull,
                    );

                    #[cfg(test)]
                    assert_eq!(intersection.is_some(), should_intersect);

                    intersection
                });
            },
        );
    }
}

// formats numbers with separators, eg 1_000_000
fn underscore_separate_number(n: impl ToString) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join("_")
}

// criterion_group!(
//     benches,
//     ray_mesh_intersection,
//     ray_mesh_intersection_no_cull,
//     ray_mesh_intersection_no_intersection,
//     ray_mesh_intersection_single_plane,
//     ray_mesh_intersection_overlapping_planes,
// );

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

    fn mesh_to_world(&self) -> Mat4 {
        Mat4::IDENTITY
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
fn bench_orig(c: &mut Criterion) {
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

macro_rules! bench_group {
    ($c: expr, $name:literal) => {{
        let mut group = $c.benchmark_group(bench!($name));

        group
            .warm_up_time(Benchmarks::WARM_UP_TIME)
            // Make the scale logarithmic, to match `VERTICES_PER_SIDE`.
            .plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

        group
    }};
}

fn bench(c: &mut Criterion) {
    cull_intersect(bench_group!(c, "cull_intersect"), true);

    no_cull_intersect(bench_group!(c, "no_cull_intersect"), true);

    cull_no_intersect(bench_group!(c, "cull_no_intersect"), false);

    single_plane(
        bench_group!(c, "single_plane_indices"),
        true,
        IntersectIndices::Include,
    );
    single_plane(
        bench_group!(c, "single_plane_no_indices"),
        true,
        IntersectIndices::Skip,
    );

    overlapping_planes(
        bench_group!(c, "overlapping_planes_back_to_front"),
        true,
        OverlappingPlaneOrdering::BackToFront,
    );
    overlapping_planes(
        bench_group!(c, "overlapping_planes_front_to_back"),
        true,
        OverlappingPlaneOrdering::FrontToBack,
    );
}
