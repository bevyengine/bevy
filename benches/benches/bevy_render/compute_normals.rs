use core::hint::black_box;

use criterion::{criterion_group, Criterion};
use rand::random;
use std::time::{Duration, Instant};

use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};

const GRID_SIZE: usize = 256;

fn compute_normals(c: &mut Criterion) {
    let indices = Indices::U32(
        (0..GRID_SIZE - 1)
            .flat_map(|i| std::iter::repeat(i).zip(0..GRID_SIZE - 1))
            .flat_map(|(i, j)| {
                let tl = ((GRID_SIZE * j) + i) as u32;
                let tr = tl + 1;
                let bl = ((GRID_SIZE * (j + 1)) + i) as u32;
                let br = bl + 1;
                [tl, bl, tr, tr, bl, br]
            })
            .collect(),
    );

    let new_mesh = || {
        let positions = (0..GRID_SIZE)
            .flat_map(|i| std::iter::repeat(i).zip(0..GRID_SIZE))
            .map(|(i, j)| [i as f32, j as f32, random::<f32>()])
            .collect::<Vec<_>>();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(indices.clone())
    };

    c.bench_function("smooth_normals", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::default();
            for _ in 0..iters {
                let mut mesh = new_mesh();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                let start = Instant::now();
                mesh.compute_smooth_normals();
                let end = Instant::now();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                total += end.duration_since(start);
            }
            total
        });
    });

    c.bench_function("angle_weighted_normals", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::default();
            for _ in 0..iters {
                let mut mesh = new_mesh();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                let start = Instant::now();
                mesh.compute_smooth_normals();
                let end = Instant::now();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                total += end.duration_since(start);
            }
            total
        });
    });

    c.bench_function("face_weighted_normals", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::default();
            for _ in 0..iters {
                let mut mesh = new_mesh();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                let start = Instant::now();
                mesh.compute_area_weighted_normals();
                let end = Instant::now();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                total += end.duration_since(start);
            }
            total
        });
    });

    let new_mesh = || new_mesh().with_duplicated_vertices();

    c.bench_function("flat_normals", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::default();
            for _ in 0..iters {
                let mut mesh = new_mesh();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                let start = Instant::now();
                mesh.compute_flat_normals();
                let end = Instant::now();
                black_box(mesh.attribute(Mesh::ATTRIBUTE_NORMAL));
                total += end.duration_since(start);
            }
            total
        });
    });
}

criterion_group!(benches, compute_normals);
