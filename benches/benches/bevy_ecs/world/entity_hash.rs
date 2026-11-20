use bevy_ecs::entity::{Entity, EntityGeneration, EntityHashSet, EntityIndex};
use chacha20::ChaCha8Rng;
use criterion::{BenchmarkId, Criterion, Throughput};
use rand::{seq::SliceRandom, RngExt, SeedableRng};

const SIZES: [usize; 5] = [100, 316, 1000, 3162, 10000];

fn make_entity(rng: &mut impl RngExt, size: usize) -> Entity {
    // -log₂(1-x) gives an exponential distribution with median 1.0
    // That lets us get values that are mostly small, but some are quite large
    // * For ids, half are in [0, size), half are unboundedly larger.
    // * For generations, half are in [1, 3), half are unboundedly larger.

    let x: f64 = rng.random();
    let id = -(1.0 - x).log2() * (size as f64);
    let x: f64 = rng.random();
    let generation = 1.0 + -(1.0 - x).log2() * 2.0;

    // this is not reliable, but we're internal so a hack is ok
    let id = id as u64 + 1;
    let bits = ((generation as u64) << 32) | id;
    let e = Entity::from_bits(bits);
    assert_eq!(e.index_u32(), !(id as u32));
    assert_eq!(
        e.generation(),
        EntityGeneration::FIRST.after_versions(generation as u32)
    );
    e
}

pub fn entity_set_build_and_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_hash");
    for size in SIZES {
        // Get some random-but-consistent entities to use for all the benches below.
        let mut rng = ChaCha8Rng::seed_from_u64(size as u64);
        let entities =
            Vec::from_iter(core::iter::repeat_with(|| make_entity(&mut rng, size)).take(size));

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

const CONTIGUOUS_SIZES: [u32; 3] = [1_000, 10_000, 100_000];

fn contiguous_entity(index: u32, versions: u32) -> Entity {
    Entity::from_index_and_generation(
        EntityIndex::from_raw_u32(index).unwrap(),
        EntityGeneration::FIRST.after_versions(versions),
    )
}

/// Freshly spawned entities have contiguous indices, which `EntityHash` keeps
/// in contiguous buckets. This benches lookups into such a densely packed set.
pub fn entity_set_contiguous(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_hash_contiguous");
    for size in CONTIGUOUS_SIZES {
        let entities = Vec::from_iter((0..size).map(|index| contiguous_entity(index, 0)));
        let set = EntityHashSet::from_iter(entities.iter().copied());

        // Look up in a random order, so this doesn't just measure locality.
        let mut rng = ChaCha8Rng::seed_from_u64(size as u64);
        let mut indices = Vec::from_iter(0..size);
        indices.shuffle(&mut rng);

        // Same indices as the set, but a later generation (despawned and reused entities).
        let stale = Vec::from_iter(indices.iter().map(|&index| contiguous_entity(index, 1)));
        // Indices that were never inserted. Some start probing inside the densely packed
        // buckets; others land on empty buckets and miss immediately.
        let absent = Vec::from_iter(
            indices
                .iter()
                .map(|&index| contiguous_entity(index + size, 0)),
        );
        let hits = Vec::from_iter(indices.iter().map(|&index| contiguous_entity(index, 0)));

        group.throughput(Throughput::Elements(size as u64));
        group.bench_function(BenchmarkId::new("build", size), |bencher| {
            bencher.iter_with_large_drop(|| EntityHashSet::from_iter(entities.iter().copied()));
        });
        for (name, lookups) in [
            ("lookup_hit", &hits),
            ("lookup_miss_gen", &stale),
            ("lookup_miss_id", &absent),
        ] {
            group.bench_function(BenchmarkId::new(name, size), |bencher| {
                bencher.iter(|| lookups.iter().filter(|e| set.contains(*e)).count());
            });
        }
    }
}
