#![no_main]

use std::collections::HashSet;

use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;
use bevy_ecs_fuzz::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum QueryOp {
    SpawnA(CompA),
    SpawnB(CompB),
    SpawnAB(CompA, CompB),
    SpawnABC(CompA, CompB, CompC),
    SpawnSparse(CompSparse),
    Despawn(u8),
    InsertA(u8, CompA),
    InsertB(u8, CompB),
    InsertC(u8, CompC),
    InsertSparse(u8, CompSparse),
    RemoveA(u8),
    RemoveB(u8),
    RemoveC(u8),
    RemoveSparse(u8),

    QueryA,
    QueryAB,
    QueryAWithB,
    QueryAWithoutB,
    QueryOptionA,
    QueryHasA,
    QuerySparse,
    QueryEntityA,
}

#[derive(Debug, Arbitrary)]
struct QueryFuzzInput {
    ops: Vec<QueryOp>,
}

struct ShadowState {
    alive: Vec<Entity>,
    has_a: HashSet<Entity>,
    has_b: HashSet<Entity>,
    has_c: HashSet<Entity>,
    has_sparse: HashSet<Entity>,
}

impl ShadowState {
    fn new() -> Self {
        Self {
            alive: Vec::new(),
            has_a: HashSet::new(),
            has_b: HashSet::new(),
            has_c: HashSet::new(),
            has_sparse: HashSet::new(),
        }
    }

    fn spawn(&mut self, e: Entity, a: bool, b: bool, c: bool, sparse: bool) {
        self.alive.push(e);
        if a {
            self.has_a.insert(e);
        }
        if b {
            self.has_b.insert(e);
        }
        if c {
            self.has_c.insert(e);
        }
        if sparse {
            self.has_sparse.insert(e);
        }
    }

    fn despawn(&mut self, idx: u8) -> Option<Entity> {
        if self.alive.is_empty() {
            return None;
        }
        let i = (idx as usize) % self.alive.len();
        let e = self.alive[i];
        self.alive.swap_remove(i);
        self.has_a.remove(&e);
        self.has_b.remove(&e);
        self.has_c.remove(&e);
        self.has_sparse.remove(&e);
        Some(e)
    }

    fn resolve(&self, idx: u8) -> Option<Entity> {
        if self.alive.is_empty() {
            None
        } else {
            Some(self.alive[(idx as usize) % self.alive.len()])
        }
    }
}

fuzz_target!(|input: QueryFuzzInput| {
    if input.ops.len() > 256 {
        return;
    }

    let mut world = World::new();
    let mut shadow = ShadowState::new();

    let mut cached_query_a = world.query::<(Entity, &CompA)>();

    for op in &input.ops {
        match op {
            QueryOp::SpawnA(a) => {
                let e = world.spawn(a.clone()).id();
                shadow.spawn(e, true, false, false, false);
            }
            QueryOp::SpawnB(b) => {
                let e = world.spawn(b.clone()).id();
                shadow.spawn(e, false, true, false, false);
            }
            QueryOp::SpawnAB(a, b) => {
                let e = world.spawn((a.clone(), b.clone())).id();
                shadow.spawn(e, true, true, false, false);
            }
            QueryOp::SpawnABC(a, b, c) => {
                let e = world.spawn((a.clone(), b.clone(), c.clone())).id();
                shadow.spawn(e, true, true, true, false);
            }
            QueryOp::SpawnSparse(s) => {
                let e = world.spawn(s.clone()).id();
                shadow.spawn(e, false, false, false, true);
            }
            QueryOp::Despawn(idx) => {
                if let Some(e) = shadow.despawn(*idx) {
                    world.despawn(e);
                }
            }
            QueryOp::InsertA(idx, a) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).insert(a.clone());
                    shadow.has_a.insert(e);
                }
            }
            QueryOp::InsertB(idx, b) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).insert(b.clone());
                    shadow.has_b.insert(e);
                }
            }
            QueryOp::InsertC(idx, c) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).insert(c.clone());
                    shadow.has_c.insert(e);
                }
            }
            QueryOp::InsertSparse(idx, s) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).insert(s.clone());
                    shadow.has_sparse.insert(e);
                }
            }
            QueryOp::RemoveA(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).remove::<CompA>();
                    shadow.has_a.remove(&e);
                }
            }
            QueryOp::RemoveB(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).remove::<CompB>();
                    shadow.has_b.remove(&e);
                }
            }
            QueryOp::RemoveC(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).remove::<CompC>();
                    shadow.has_c.remove(&e);
                }
            }
            QueryOp::RemoveSparse(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).remove::<CompSparse>();
                    shadow.has_sparse.remove(&e);
                }
            }

            QueryOp::QueryA => {
                let mut q = world.query::<(Entity, &CompA)>();
                let results: HashSet<Entity> = q.iter(&world).map(|(e, _)| e).collect();
                assert_eq!(results, shadow.has_a, "Query<&CompA> mismatch");
            }
            QueryOp::QueryAB => {
                let mut q = world.query::<(Entity, &CompA, &CompB)>();
                let results: HashSet<Entity> = q.iter(&world).map(|(e, _, _)| e).collect();
                let expected: HashSet<Entity> =
                    shadow.has_a.intersection(&shadow.has_b).copied().collect();
                assert_eq!(results, expected, "Query<(&CompA, &CompB)> mismatch");
            }
            QueryOp::QueryAWithB => {
                let mut q = world.query_filtered::<(Entity, &CompA), With<CompB>>();
                let results: HashSet<Entity> = q.iter(&world).map(|(e, _)| e).collect();
                let expected: HashSet<Entity> =
                    shadow.has_a.intersection(&shadow.has_b).copied().collect();
                assert_eq!(results, expected, "Query<&CompA, With<CompB>> mismatch");
            }
            QueryOp::QueryAWithoutB => {
                let mut q = world.query_filtered::<(Entity, &CompA), Without<CompB>>();
                let results: HashSet<Entity> = q.iter(&world).map(|(e, _)| e).collect();
                let expected: HashSet<Entity> =
                    shadow.has_a.difference(&shadow.has_b).copied().collect();
                assert_eq!(results, expected, "Query<&CompA, Without<CompB>> mismatch");
            }
            QueryOp::QueryOptionA => {
                let mut q = world.query::<(Entity, Option<&CompA>)>();
                let alive_set: HashSet<Entity> = shadow.alive.iter().copied().collect();
                for (e, opt_a) in q.iter(&world) {
                    // Only validate entities we track (world has extra resource entities)
                    if alive_set.contains(&e) {
                        assert_eq!(
                            opt_a.is_some(),
                            shadow.has_a.contains(&e),
                            "Option<&CompA> mismatch for {e:?}"
                        );
                    }
                }
            }
            QueryOp::QueryHasA => {
                let mut q = world.query::<(Entity, Has<CompA>)>();
                let alive_set: HashSet<Entity> = shadow.alive.iter().copied().collect();
                for (e, has) in q.iter(&world) {
                    if alive_set.contains(&e) {
                        assert_eq!(
                            has,
                            shadow.has_a.contains(&e),
                            "Has<CompA> mismatch for {e:?}"
                        );
                    }
                }
            }
            QueryOp::QuerySparse => {
                let mut q = world.query::<(Entity, &CompSparse)>();
                let results: HashSet<Entity> = q.iter(&world).map(|(e, _)| e).collect();
                assert_eq!(results, shadow.has_sparse, "Query<&CompSparse> mismatch");
            }
            QueryOp::QueryEntityA => {
                let results: HashSet<Entity> =
                    cached_query_a.iter(&world).map(|(e, _)| e).collect();
                assert_eq!(
                    results, shadow.has_a,
                    "Cached Query<&CompA> mismatch (archetype cache invalidation bug?)"
                );
            }
        }
    }

    check_world_invariants(&mut world, &shadow.alive);
});
