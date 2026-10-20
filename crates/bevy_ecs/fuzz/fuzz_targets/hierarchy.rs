#![no_main]

use std::collections::{HashMap, HashSet};

use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;
use bevy_ecs_fuzz::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
pub enum HierarchyOp {
    SpawnEmpty,
    SpawnWithA(CompA),
    Despawn(u8),

    SetParent(u8, u8),
    RemoveParent(u8),
    AddChild(u8, u8),
    SpawnChild(u8),
    DetachAllChildren(u8),

    InsertA(u8, CompA),
    RemoveA(u8),

    CheckInvariants,
}

#[derive(Debug, Arbitrary)]
struct HierarchyFuzzInput {
    ops: Vec<HierarchyOp>,
}

struct Shadow {
    alive: Vec<Entity>,
    parent_of: HashMap<Entity, Entity>,
}

impl Shadow {
    fn new() -> Self {
        Self {
            alive: Vec::new(),
            parent_of: HashMap::new(),
        }
    }

    fn resolve(&self, idx: u8) -> Option<Entity> {
        if self.alive.is_empty() {
            None
        } else {
            Some(self.alive[(idx as usize) % self.alive.len()])
        }
    }

    fn spawn(&mut self, e: Entity) {
        self.alive.push(e);
    }

    fn despawn(&mut self, idx: u8) -> Option<Entity> {
        if self.alive.is_empty() {
            return None;
        }
        let i = (idx as usize) % self.alive.len();
        let e = self.alive[i];
        let mut visited = HashSet::new();
        self.despawn_recursive(e, &mut visited);
        Some(e)
    }

    fn despawn_recursive(&mut self, e: Entity, visited: &mut HashSet<Entity>) {
        if !visited.insert(e) {
            return; // Already visited, avoid cycles
        }

        let children: Vec<Entity> = self
            .parent_of
            .iter()
            .filter(|(_, parent)| **parent == e)
            .map(|(child, _)| *child)
            .collect();

        for child in children {
            self.despawn_recursive(child, visited);
        }

        if let Some(pos) = self.alive.iter().position(|&x| x == e) {
            self.alive.swap_remove(pos);
        }
        self.parent_of.remove(&e);
    }

    fn set_parent(&mut self, child: Entity, parent: Entity) {
        if child == parent {
            return;
        }
        self.parent_of.insert(child, parent);
    }

    fn remove_parent(&mut self, child: Entity) {
        self.parent_of.remove(&child);
    }

    fn children_of(&self, parent: Entity) -> Vec<Entity> {
        self.parent_of
            .iter()
            .filter(|(_, p)| **p == parent)
            .map(|(c, _)| *c)
            .collect()
    }
}

fn check_hierarchy_invariants(world: &mut World, shadow: &Shadow) {
    for (&child, &parent) in &shadow.parent_of {
        if !shadow.alive.contains(&child) || !shadow.alive.contains(&parent) {
            continue;
        }

        let child_ref = world.entity(child);
        let child_of = child_ref.get::<ChildOf>();
        assert!(
            child_of.is_some(),
            "Entity {child:?} should have ChildOf({parent:?}) but doesn't"
        );
        assert_eq!(
            child_of.unwrap().parent(),
            parent,
            "Entity {child:?} has wrong parent: expected {parent:?}, got {:?}",
            child_of.unwrap().parent()
        );
    }

    for &e in &shadow.alive {
        if shadow.parent_of.contains_key(&e) {
            continue;
        }
        let entity_ref = world.entity(e);
        assert!(
            entity_ref.get::<ChildOf>().is_none(),
            "Entity {e:?} should NOT have ChildOf but does: {:?}",
            entity_ref.get::<ChildOf>()
        );
    }

    for &e in &shadow.alive {
        let entity_ref = world.entity(e);
        if let Some(child_of) = entity_ref.get::<ChildOf>() {
            let parent = child_of.parent();
            if let Ok(parent_ref) = world.get_entity(parent) {
                if let Some(children) = parent_ref.get::<Children>() {
                    assert!(
                        children.iter().any(|c| c == e),
                        "Entity {e:?} has ChildOf({parent:?}) but parent's Children doesn't contain it"
                    );
                } else {
                    panic!(
                        "Entity {e:?} has ChildOf({parent:?}) but parent has no Children component"
                    );
                }
            }
        }
    }

    for &e in &shadow.alive {
        let entity_ref = world.entity(e);
        if let Some(children) = entity_ref.get::<Children>() {
            let child_list: Vec<Entity> = children.iter().collect();
            let unique: HashSet<Entity> = child_list.iter().copied().collect();
            assert_eq!(
                child_list.len(),
                unique.len(),
                "Entity {e:?} has duplicate children"
            );

            for &child in &child_list {
                if let Ok(child_ref) = world.get_entity(child) {
                    let child_of = child_ref.get::<ChildOf>();
                    assert!(
                        child_of.is_some(),
                        "Entity {child:?} is in {e:?}'s Children but has no ChildOf"
                    );
                    assert_eq!(
                        child_of.unwrap().parent(),
                        e,
                        "Entity {child:?} is in {e:?}'s Children but ChildOf points to {:?}",
                        child_of.unwrap().parent()
                    );
                }
            }
        }
    }
}

fuzz_target!(|input: HierarchyFuzzInput| {
    if input.ops.len() > 256 {
        return;
    }

    let mut world = World::new();
    let mut shadow = Shadow::new();

    for op in &input.ops {
        match op {
            HierarchyOp::SpawnEmpty => {
                let e = world.spawn_empty().id();
                shadow.spawn(e);
            }
            HierarchyOp::SpawnWithA(a) => {
                let e = world.spawn(a.clone()).id();
                shadow.spawn(e);
            }

            HierarchyOp::Despawn(idx) => {
                if let Some(e) = shadow.despawn(*idx) {
                    world.despawn(e);
                }
            }

            HierarchyOp::SetParent(child_idx, parent_idx) => {
                if let (Some(child), Some(parent)) =
                    (shadow.resolve(*child_idx), shadow.resolve(*parent_idx))
                    && child != parent
                {
                    shadow.set_parent(child, parent);
                    world.entity_mut(child).insert(ChildOf(parent));
                }
            }

            HierarchyOp::RemoveParent(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    shadow.remove_parent(e);
                    world.entity_mut(e).remove::<ChildOf>();
                }
            }

            HierarchyOp::AddChild(parent_idx, child_idx) => {
                if let (Some(parent), Some(child)) =
                    (shadow.resolve(*parent_idx), shadow.resolve(*child_idx))
                    && child != parent
                {
                    shadow.set_parent(child, parent);
                    world.entity_mut(parent).add_child(child);
                }
            }

            HierarchyOp::SpawnChild(parent_idx) => {
                if let Some(parent) = shadow.resolve(*parent_idx) {
                    world.entity_mut(parent).with_child(CompA(0));
                    let entity_ref = world.entity(parent);
                    if let Some(children) = entity_ref.get::<Children>()
                        && let Some(last_child) = children.last()
                        && !shadow.alive.contains(last_child)
                    {
                        shadow.spawn(*last_child);
                        shadow.set_parent(*last_child, parent);
                    }
                }
            }

            HierarchyOp::DetachAllChildren(idx) => {
                if let Some(parent) = shadow.resolve(*idx) {
                    let children = shadow.children_of(parent);
                    for child in children {
                        shadow.remove_parent(child);
                    }
                    world.entity_mut(parent).detach_all_children();
                }
            }

            HierarchyOp::InsertA(idx, a) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).insert(a.clone());
                }
            }
            HierarchyOp::RemoveA(idx) => {
                if let Some(e) = shadow.resolve(*idx) {
                    world.entity_mut(e).remove::<CompA>();
                }
            }

            HierarchyOp::CheckInvariants => {
                check_world_invariants(&mut world, &shadow.alive);
                check_hierarchy_invariants(&mut world, &shadow);
            }
        }
    }

    // Always check invariants at the end
    check_world_invariants(&mut world, &shadow.alive);
    check_hierarchy_invariants(&mut world, &shadow);
});
