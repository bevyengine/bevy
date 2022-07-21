use bevy_asset::{AssetPath, Handle, HandleId, HandleMap};
use bevy_reflect::TypeUuid;
use bevy_utils::HashMap;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::prelude::*;

const NUM_UNIQUE_IDS: u64 = 10000;
const NUM_GET: usize = 1000000;

#[derive(TypeUuid)]
#[uuid = "c976f5a3-a461-459e-bfb6-424e9e72f708"]
struct TestAsset {}

fn bench_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("Handle Map");

    let mut rng = thread_rng();

    for ratio in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0] {
        let handles: Vec<_> = (0..NUM_UNIQUE_IDS)
            .map(|i| {
                let id = if i < (NUM_UNIQUE_IDS as f32 * ratio) as u64 {
                    HandleId::Id(TestAsset::TYPE_UUID, i)
                } else {
                    HandleId::AssetPathId(
                        AssetPath::new(format!("Asset Path Id Number: {}", i).into(), None)
                            .get_id(),
                    )
                };

                Handle::<TestAsset>::weak(id)
            })
            .collect();
        let access_order: Vec<_> = (0..NUM_GET)
            .map(|_| rng.gen_range(0..NUM_UNIQUE_IDS) as usize)
            .collect();

        group.bench_with_input(
            BenchmarkId::new("HashMap", ratio),
            &handles,
            |b, handles| {
                b.iter(|| {
                    let mut map: HashMap<Handle<TestAsset>, u64> = HashMap::with_capacity(10000);

                    for (i, handle) in handles.iter().enumerate() {
                        map.insert(handle.clone_weak(), i as u64);
                    }

                    for handle_idx in &access_order {
                        black_box(map.get(&handles[*handle_idx]));
                    }
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("HandleMap", ratio),
            &handles,
            |b, handles| {
                b.iter(|| {
                    let mut map: HandleMap<TestAsset, u64> = HandleMap::with_capacity(5000, 5000);

                    for (i, handle) in handles.iter().enumerate() {
                        map.insert(handle, i as u64);
                    }

                    for handle_idx in &access_order {
                        black_box(map.get(&handles[*handle_idx]));
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_overhead);
criterion_main!(benches);
