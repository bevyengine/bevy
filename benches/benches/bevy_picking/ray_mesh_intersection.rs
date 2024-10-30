use bevy_math::{Dir3, Mat4, Ray3d, Vec3};
use bevy_picking::mesh_picking::ray_cast;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

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

fn ray_mesh_intersection(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_mesh_intersection");
    group.warm_up_time(std::time::Duration::from_millis(500));

    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = mesh_creation(vertices_per_side);
        let tri_count = (mesh.indices.len() / 3) as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
                underscore_separate_number(mesh.indices.len() / 3)
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    black_box(ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Some(&mesh.indices),
                        ray_cast::Backfaces::Cull,
                    ));
                });
            },
        );
    }
}

fn ray_mesh_intersection_no_cull(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_mesh_intersection_no_cull");
    group.warm_up_time(std::time::Duration::from_millis(500));

    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = mesh_creation(vertices_per_side);
        let tri_count = (mesh.indices.len() / 3) as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
                underscore_separate_number(mesh.indices.len() / 3)
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    black_box(ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Some(&mesh.indices),
                        ray_cast::Backfaces::Include,
                    ));
                });
            },
        );
    }
}

fn ray_mesh_intersection_no_intersection(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_mesh_intersection_no_intersection");
    group.warm_up_time(std::time::Duration::from_millis(500));

    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = mesh_creation(vertices_per_side);
        let tri_count = (mesh.indices.len() / 3) as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions, {} indices)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
                underscore_separate_number(mesh.indices.len() / 3)
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    black_box(ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Some(&mesh.indices),
                        ray_cast::Backfaces::Cull,
                    ));
                });
            },
        );
    }
}

fn ray_mesh_intersection_no_indices(c: &mut Criterion) {
    let mut group = c.benchmark_group("ray_mesh_intersection_no_indices");
    group.warm_up_time(std::time::Duration::from_millis(500));

    for vertices_per_side in [10_u32, 100, 1000] {
        let ray = Ray3d::new(Vec3::new(0.0, 1.0, 0.0), Dir3::NEG_Y);
        let mesh_to_world = Mat4::IDENTITY;
        let mesh = mesh_creation(vertices_per_side);
        let tri_count = mesh.positions.len() as u64;

        group.throughput(Throughput::Elements(tri_count));
        group.bench_with_input(
            format!(
                "{} triangles ({} positions)",
                underscore_separate_number(tri_count),
                underscore_separate_number(mesh.positions.len()),
            ),
            &(ray, mesh_to_world, mesh),
            |b, (ray, mesh_to_world, mesh)| {
                b.iter(|| {
                    black_box(ray_cast::ray_mesh_intersection(
                        *ray,
                        mesh_to_world,
                        &mesh.positions,
                        Some(&mesh.normals),
                        Option::<&[u32]>::None,
                        ray_cast::Backfaces::Cull,
                    ));
                });
            },
        );
    }
}

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

criterion_group!(
    benches,
    ray_mesh_intersection,
    ray_mesh_intersection_no_cull,
    ray_mesh_intersection_no_intersection,
    ray_mesh_intersection_no_indices,
);
criterion_main!(benches);
