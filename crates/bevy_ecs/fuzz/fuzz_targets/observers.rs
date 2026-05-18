#![no_main]
#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;
use bevy_ecs_fuzz::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum ObserverOp {
    SpawnEmpty,
    SpawnA(CompA),
    SpawnB(CompB),
    SpawnAB(CompA, CompB),
    Despawn(u8),
    InsertA(u8, CompA),
    InsertB(u8, CompB),
    InsertC(u8, CompC),
    RemoveA(u8),
    RemoveB(u8),
    RemoveC(u8),
    ReplaceA(u8, CompA),

    TriggerCustom(u32),

    DespawnObserver(u8),

    CheckCounts,
}

#[derive(Debug, Arbitrary)]
struct ObserverFuzzInput {
    ops: Vec<ObserverOp>,
}

#[derive(Resource, Clone)]
struct ObserverCounts {
    add_a: Arc<AtomicU32>,
    insert_a: Arc<AtomicU32>,
    discard_a: Arc<AtomicU32>,
    remove_a: Arc<AtomicU32>,
    add_b: Arc<AtomicU32>,
    insert_b: Arc<AtomicU32>,
    remove_b: Arc<AtomicU32>,
    custom: Arc<AtomicU32>,
}

impl Default for ObserverCounts {
    fn default() -> Self {
        Self {
            add_a: Arc::new(AtomicU32::new(0)),
            insert_a: Arc::new(AtomicU32::new(0)),
            discard_a: Arc::new(AtomicU32::new(0)),
            remove_a: Arc::new(AtomicU32::new(0)),
            add_b: Arc::new(AtomicU32::new(0)),
            insert_b: Arc::new(AtomicU32::new(0)),
            remove_b: Arc::new(AtomicU32::new(0)),
            custom: Arc::new(AtomicU32::new(0)),
        }
    }
}

#[derive(Event)]
struct CustomEvent(u32);

struct Shadow {
    alive: Vec<Entity>,
    observer_entities: Vec<Entity>,
    has_a: std::collections::HashSet<Entity>,
    has_b: std::collections::HashSet<Entity>,
    has_c: std::collections::HashSet<Entity>,
    expected_add_a: u32,
    expected_insert_a: u32,
    expected_discard_a: u32,
    expected_remove_a: u32,
    expected_add_b: u32,
    expected_insert_b: u32,
    expected_remove_b: u32,
    expected_custom: u32,
}

impl Shadow {
    fn new() -> Self {
        Self {
            alive: Vec::new(),
            observer_entities: Vec::new(),
            has_a: std::collections::HashSet::new(),
            has_b: std::collections::HashSet::new(),
            has_c: std::collections::HashSet::new(),
            expected_add_a: 0,
            expected_insert_a: 0,
            expected_discard_a: 0,
            expected_remove_a: 0,
            expected_add_b: 0,
            expected_insert_b: 0,
            expected_remove_b: 0,
            expected_custom: 0,
        }
    }

    fn resolve(&self, idx: u8) -> Option<Entity> {
        if self.alive.is_empty() {
            None
        } else {
            Some(self.alive[(idx as usize) % self.alive.len()])
        }
    }

    fn resolve_observer(&self, idx: u8) -> Option<(usize, Entity)> {
        if self.observer_entities.is_empty() {
            None
        } else {
            let i = (idx as usize) % self.observer_entities.len();
            Some((i, self.observer_entities[i]))
        }
    }

    fn despawn(&mut self, idx: u8) -> Option<Entity> {
        if self.alive.is_empty() {
            return None;
        }
        let i = (idx as usize) % self.alive.len();
        let e = self.alive[i];

        if self.has_a.remove(&e) {
            self.expected_discard_a += 1;
            self.expected_remove_a += 1;
        }
        if self.has_b.remove(&e) {
            self.expected_remove_b += 1;
        }
        self.has_c.remove(&e);

        self.alive.swap_remove(i);
        Some(e)
    }

    fn spawn(&mut self, e: Entity, a: bool, b: bool) {
        self.alive.push(e);
        if a {
            self.has_a.insert(e);
            self.expected_add_a += 1;
            self.expected_insert_a += 1;
        }
        if b {
            self.has_b.insert(e);
            self.expected_add_b += 1;
            self.expected_insert_b += 1;
        }
    }

    fn insert_a(&mut self, e: Entity) {
        if !self.has_a.contains(&e) {
            self.expected_add_a += 1;
        } else {
            self.expected_discard_a += 1;
        }
        self.expected_insert_a += 1;
        self.has_a.insert(e);
    }

    fn insert_b(&mut self, e: Entity) {
        if !self.has_b.contains(&e) {
            self.expected_add_b += 1;
        }
        self.expected_insert_b += 1;
        self.has_b.insert(e);
    }

    fn remove_a(&mut self, e: Entity) {
        if self.has_a.remove(&e) {
            self.expected_discard_a += 1;
            self.expected_remove_a += 1;
        }
    }

    fn remove_b(&mut self, e: Entity) {
        if self.has_b.remove(&e) {
            self.expected_remove_b += 1;
        }
    }
}

fuzz_target!(|input: ObserverFuzzInput| {
    if input.ops.len() > 256 {
        return;
    }

    let mut world = World::new();
    let counts = ObserverCounts::default();
    world.insert_resource(counts.clone());

    let mut shadow = Shadow::new();

    {
        let c = counts.add_a.clone();
        let e = world
            .add_observer(move |_: On<Add, CompA>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }
    {
        let c = counts.insert_a.clone();
        let e = world
            .add_observer(move |_: On<Insert, CompA>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }
    {
        let c = counts.discard_a.clone();
        let e = world
            .add_observer(move |_: On<Discard, CompA>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }
    {
        let c = counts.remove_a.clone();
        let e = world
            .add_observer(move |_: On<Remove, CompA>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }

    {
        let c = counts.add_b.clone();
        let e = world
            .add_observer(move |_: On<Add, CompB>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }
    {
        let c = counts.insert_b.clone();
        let e = world
            .add_observer(move |_: On<Insert, CompB>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }
    {
        let c = counts.remove_b.clone();
        let e = world
            .add_observer(move |_: On<Remove, CompB>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }

    {
        let c = counts.custom.clone();
        let e = world
            .add_observer(move |_: On<CustomEvent>| {
                c.fetch_add(1, Ordering::Relaxed);
            })
            .id();
        shadow.observer_entities.push(e);
    }

    let e = world
        .add_observer(|_: On<Add, CompC>, mut commands: Commands| {
            commands.spawn_empty();
        })
        .id();
    shadow.observer_entities.push(e);

    let initial_observer_count = shadow.observer_entities.len();

    for op in &input.ops {
        match op {
            ObserverOp::SpawnEmpty => {
                let e = world.spawn_empty().id();
                shadow.spawn(e, false, false);
            }
            ObserverOp::SpawnA(a) => {
                let e = world.spawn(a.clone()).id();
                shadow.spawn(e, true, false);
            }
            ObserverOp::SpawnB(b) => {
                let e = world.spawn(b.clone()).id();
                shadow.spawn(e, false, true);
            }
            ObserverOp::SpawnAB(a, b) => {
                let e = world.spawn((a.clone(), b.clone())).id();
                shadow.spawn(e, true, true);
            }

            ObserverOp::Despawn(idx) => {
                if let Some(e) = shadow.despawn(*idx) {
                    world.despawn(e);
                }
            }

            ObserverOp::InsertA(idx, a) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.insert_a(e);
                    world.entity_mut(e).insert(a.clone());
                }
            }
            ObserverOp::InsertB(idx, b) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.insert_b(e);
                    world.entity_mut(e).insert(b.clone());
                }
            }
            ObserverOp::InsertC(idx, c) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.has_c.insert(e);
                    world.entity_mut(e).insert(c.clone());
                }
            }

            ObserverOp::RemoveA(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.remove_a(e);
                    world.entity_mut(e).remove::<CompA>();
                }
            }
            ObserverOp::RemoveB(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.remove_b(e);
                    world.entity_mut(e).remove::<CompB>();
                }
            }
            ObserverOp::RemoveC(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.has_c.remove(&e);
                    world.entity_mut(e).remove::<CompC>();
                }
            }

            ObserverOp::ReplaceA(idx, a) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.insert_a(e);
                    world.entity_mut(e).insert(a.clone());
                }
            }

            ObserverOp::TriggerCustom(v) => {
                shadow.expected_custom += 1;
                world.trigger(CustomEvent(*v));
            }

            ObserverOp::DespawnObserver(idx) => {
                if let Some((i, e)) = shadow.resolve_observer(*idx) {
                    if world.get_entity(e).is_ok() {
                        world.despawn(e);
                    }
                    shadow.observer_entities.swap_remove(i);
                }
            }

            ObserverOp::CheckCounts => {
                check_world_invariants(&mut world, &shadow.alive);
                if shadow.observer_entities.len() == initial_observer_count {
                    check_observer_counts(&counts, &shadow);
                }
            }
        }
    }

    check_world_invariants(&mut world, &shadow.alive);
    if shadow.observer_entities.len() == initial_observer_count {
        check_observer_counts(&counts, &shadow);
    }
});

fn check_observer_counts(counts: &ObserverCounts, shadow: &Shadow) {
    assert_eq!(
        counts.add_a.load(Ordering::Relaxed),
        shadow.expected_add_a,
        "Add<CompA> count mismatch"
    );
    assert_eq!(
        counts.insert_a.load(Ordering::Relaxed),
        shadow.expected_insert_a,
        "Insert<CompA> count mismatch"
    );
    assert_eq!(
        counts.discard_a.load(Ordering::Relaxed),
        shadow.expected_discard_a,
        "Discard<CompA> count mismatch"
    );
    assert_eq!(
        counts.remove_a.load(Ordering::Relaxed),
        shadow.expected_remove_a,
        "Remove<CompA> count mismatch"
    );
    assert_eq!(
        counts.add_b.load(Ordering::Relaxed),
        shadow.expected_add_b,
        "Add<CompB> count mismatch"
    );
    assert_eq!(
        counts.insert_b.load(Ordering::Relaxed),
        shadow.expected_insert_b,
        "Insert<CompB> count mismatch"
    );
    assert_eq!(
        counts.remove_b.load(Ordering::Relaxed),
        shadow.expected_remove_b,
        "Remove<CompB> count mismatch"
    );
    assert_eq!(
        counts.custom.load(Ordering::Relaxed),
        shadow.expected_custom,
        "CustomEvent count mismatch"
    );
}
