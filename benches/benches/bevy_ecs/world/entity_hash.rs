use bevy_ecs::entity::{Entity, EntityHashSet};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

criterion_group!(benches, entity_set_build_and_lookup,);
criterion_main!(benches);

const SIZES: [usize; 5] = [100, 316, 1000, 3162, 10000];

fn make_entity(rng: &mut impl Rng, size: usize) -> Entity {
    // -log₂(1-x) gives an exponential distribution with median 1.0
    // That lets us get values that are mostly small, but some are quite large
    // * For ids, half are in [0, size), half are unboundedly larger.
    // * For generations, half are in [1, 3), half are unboundedly larger.

    let x: f64 = rng.gen();
    let id = -(1.0 - x).log2() * (size as f64);
    let x: f64 = rng.gen();
    let gen = 1.0 + -(1.0 - x).log2() * 2.0;

    // this is not reliable, but we're internal so a hack is ok
    let bits = ((gen as u64) << 32) | (id as u64);
    let e = Entity::from_bits(bits);
    assert_eq!(e.index(), id as u32);
    assert_eq!(e.generation(), gen as u32);
    e
}

pub fn entity_set_build_and_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_hash");
    for size in SIZES {
        // Get some random-but-consistent entities to use for all the benches below.
        let mut rng = ChaCha8Rng::seed_from_u64(size as u64);
        let entities =
            Vec::from_iter(std::iter::repeat_with(|| make_entity(&mut rng, size)).take(size));

        group.throughput(Throughput::Elements(size as u64));
        group.bench_function(BenchmarkId::new("entity_set_build", size), |bencher| {
            bencher.iter_with_large_drop(|| EntityHashSet::from_iter(entities.iter().copied()));
        });
        group.bench_function(BenchmarkId::new("entity_set_lookup_hit", size), |bencher| {
            let set = EntityHashSet::from_iter(entities.iter().copied());
            bencher.iter(|| entities.iter().copied().filter(|e| set.contains(e)).count());
        });
        group.bench_function(
            BenchmarkId::new("entity_set_lookup_miss_id", size),
            |bencher| {
                let set = EntityHashSet::from_iter(entities.iter().copied());
                bencher.iter(|| {
                    entities
                        .iter()
                        .copied()
                        .map(|e| Entity::from_bits(e.to_bits() + 1))
                        .filter(|e| set.contains(e))
                        .count()
                });
            },
        );
        group.bench_function(
            BenchmarkId::new("entity_set_lookup_miss_gen", size),
            |bencher| {
                let set = EntityHashSet::from_iter(entities.iter().copied());
                bencher.iter(|| {
                    entities
                        .iter()
                        .copied()
                        .map(|e| Entity::from_bits(e.to_bits() + (1 << 32)))
                        .filter(|e| set.contains(e))
                        .count()
                });
            },
        );
    }
}
