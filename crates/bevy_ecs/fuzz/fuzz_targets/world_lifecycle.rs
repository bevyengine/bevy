#![no_main]

use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;
use bevy_ecs_fuzz::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum WorldOp {
    SpawnEmpty,
    SpawnA(CompA),
    SpawnB(CompB),
    SpawnAB(CompA, CompB),
    SpawnABC(CompA, CompB, CompC),
    SpawnSparse(CompSparse),
    SpawnMarker,
    SpawnAll(CompA, CompB, CompC, CompSparse),

    Despawn(u8),

    InsertA(u8, CompA),
    InsertB(u8, CompB),
    InsertC(u8, CompC),
    InsertSparse(u8, CompSparse),
    InsertMarker(u8),

    RemoveA(u8),
    RemoveB(u8),
    RemoveC(u8),
    RemoveSparse(u8),
    RemoveMarker(u8),

    CheckInvariants,
}

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    ops: Vec<WorldOp>,
}

fuzz_target!(|input: FuzzInput| {
    if input.ops.len() > 256 {
        return;
    }

    let mut world = World::new();
    let mut alive: Vec<Entity> = Vec::new();

    for op in &input.ops {
        match op {
            WorldOp::SpawnEmpty => {
                alive.push(world.spawn_empty().id());
            }
            WorldOp::SpawnA(a) => {
                alive.push(world.spawn(a.clone()).id());
            }
            WorldOp::SpawnB(b) => {
                alive.push(world.spawn(b.clone()).id());
            }
            WorldOp::SpawnAB(a, b) => {
                alive.push(world.spawn((a.clone(), b.clone())).id());
            }
            WorldOp::SpawnABC(a, b, c) => {
                alive.push(world.spawn((a.clone(), b.clone(), c.clone())).id());
            }
            WorldOp::SpawnSparse(s) => {
                alive.push(world.spawn(s.clone()).id());
            }
            WorldOp::SpawnMarker => {
                alive.push(world.spawn(Marker).id());
            }
            WorldOp::SpawnAll(a, b, c, s) => {
                alive.push(
                    world
                        .spawn((a.clone(), b.clone(), c.clone(), s.clone(), Marker))
                        .id(),
                );
            }

            WorldOp::Despawn(idx) => {
                if let Some((i, e)) = resolve_idx(*idx, &alive) {
                    world.despawn(e);
                    alive.swap_remove(i);
                }
            }

            WorldOp::InsertA(idx, a) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).insert(a.clone());
                }
            }
            WorldOp::InsertB(idx, b) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).insert(b.clone());
                }
            }
            WorldOp::InsertC(idx, c) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).insert(c.clone());
                }
            }
            WorldOp::InsertSparse(idx, s) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).insert(s.clone());
                }
            }
            WorldOp::InsertMarker(idx) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).insert(Marker);
                }
            }

            WorldOp::RemoveA(idx) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).remove::<CompA>();
                }
            }
            WorldOp::RemoveB(idx) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).remove::<CompB>();
                }
            }
            WorldOp::RemoveC(idx) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).remove::<CompC>();
                }
            }
            WorldOp::RemoveSparse(idx) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).remove::<CompSparse>();
                }
            }
            WorldOp::RemoveMarker(idx) => {
                if let Some((_, e)) = resolve_idx(*idx, &alive) {
                    world.entity_mut(e).remove::<Marker>();
                }
            }

            WorldOp::CheckInvariants => {
                check_world_invariants(&mut world, &alive);
            }
        }
    }

    // Always check invariants at the end
    check_world_invariants(&mut world, &alive);
});

pub fn resolve_idx(idx: u8, alive: &[Entity]) -> Option<(usize, Entity)> {
    if alive.is_empty() {
        None
    } else {
        let i = (idx as usize) % alive.len();
        Some((i, alive[i]))
    }
}
