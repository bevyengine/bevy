use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use core::hint::black_box;
use criterion::*;
use glam::*;

criterion_group!(benches, iter_frag_empty);

#[derive(Component, Default)]
struct Table<const X: usize = 0>(usize);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct Sparse<const X: usize = 0>(usize);

fn flip_coin() -> bool {
    rand::random::<bool>()
}
fn iter_frag_empty(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter_fragmented(4096)_empty");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    group.bench_function("foreach_table", |b| {
        let mut world = World::new();
        spawn_empty_frag_archetype::<Table>(&mut world);
        let mut q: SystemState<Query<(Entity, &Table)>> =
            SystemState::<Query<(Entity, &Table<0>)>>::new(&mut world);
        let query = q.get(&world);
        b.iter(move || {
            let mut res = 0;
            query.iter().for_each(|(e, t)| {
                res += e.to_bits();
                black_box(t);
            });
        });
    });
    group.bench_function("foreach_sparse", |b| {
        let mut world = World::new();
        spawn_empty_frag_archetype::<Sparse>(&mut world);
        let mut q: SystemState<Query<(Entity, &Sparse)>> =
            SystemState::<Query<(Entity, &Sparse<0>)>>::new(&mut world);
        let query = q.get(&world);
        b.iter(move || {
            let mut res = 0;
            query.iter().for_each(|(e, t)| {
                res += e.to_bits();
                black_box(t);
            });
        });
    });
    group.finish();

    fn spawn_empty_frag_archetype<T: Component + Default>(world: &mut World) {
        for i in 0..65536 {
            let mut e = world.spawn_empty();
            if flip_coin() {
                e.insert(Table::<1>(0));
            }
            if flip_coin() {
                e.insert(Table::<2>(0));
            }
            if flip_coin() {
                e.insert(Table::<3>(0));
            }
            if flip_coin() {
                e.insert(Table::<4>(0));
            }
            if flip_coin() {
                e.insert(Table::<5>(0));
            }
            if flip_coin() {
                e.insert(Table::<6>(0));
            }
            if flip_coin() {
                e.insert(Table::<7>(0));
            }
            if flip_coin() {
                e.insert(Table::<8>(0));
            }
            if flip_coin() {
                e.insert(Table::<9>(0));
            }
            if flip_coin() {
                e.insert(Table::<10>(0));
            }
            if flip_coin() {
                e.insert(Table::<11>(0));
            }
            if flip_coin() {
                e.insert(Table::<12>(0));
            }
            e.insert(T::default());

            if i != 0 {
                e.despawn();
            }
        }
    }
}
