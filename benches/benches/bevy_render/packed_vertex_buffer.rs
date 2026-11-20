use core::hint::black_box;

use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use wgpu_types::WriteOnly;

use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Mesh, PrimitiveTopology};

fn pbr_mesh(vertex_count: usize) -> Mesh {
    let f = |i: usize, k: usize| (i * 4 + k) as f32;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        (0..vertex_count)
            .map(|i| [f(i, 0), f(i, 1), f(i, 2)])
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        (0..vertex_count)
            .map(|i| [f(i, 3), f(i, 4), f(i, 5)])
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        (0..vertex_count)
            .map(|i| [f(i, 6), f(i, 7)])
            .collect::<Vec<_>>(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_TANGENT,
        (0..vertex_count)
            .map(|i| [f(i, 8), f(i, 9), f(i, 10), f(i, 11)])
            .collect::<Vec<_>>(),
    )
}

fn skinned_mesh(vertex_count: usize) -> Mesh {
    pbr_mesh(vertex_count)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            bevy_mesh::VertexAttributeValues::Uint16x4(
                (0..vertex_count)
                    .map(|i| [i as u16, (i + 1) as u16, (i + 2) as u16, (i + 3) as u16])
                    .collect(),
            ),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            (0..vertex_count)
                .map(|_| [0.4f32, 0.3, 0.2, 0.1])
                .collect::<Vec<_>>(),
        )
}

fn packed_vertex_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_packed_vertex_buffer_data");

    for (name, make_mesh) in [
        ("pbr", pbr_mesh as fn(usize) -> Mesh),
        ("skinned", skinned_mesh),
    ] {
        for vertex_count in [1_000, 65_536, 1_048_576] {
            let mesh = make_mesh(vertex_count);
            let mut buffer = vec![0u8; mesh.get_vertex_buffer_size()];

            group.throughput(Throughput::Elements(vertex_count as u64));
            group.bench_function(BenchmarkId::new(name, vertex_count), |b| {
                b.iter(|| {
                    black_box(&mesh).write_packed_vertex_buffer_data(WriteOnly::from_mut(
                        black_box(buffer.as_mut_slice()),
                    ));
                });
            });
        }
    }

    group.finish();
}

criterion_group!(benches, packed_vertex_buffer);
