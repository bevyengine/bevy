use arbitrary::Arbitrary;
use bevy_ecs::prelude::*;

#[derive(Component, Clone, Debug, Arbitrary)]
pub struct CompA(pub u32);

#[derive(Component, Clone, Debug, Arbitrary)]
pub struct CompB(pub u64);

#[derive(Component, Clone, Debug, Arbitrary)]
pub struct CompC(pub i16);

#[derive(Component, Clone, Debug, Arbitrary)]
#[component(storage = "SparseSet")]
pub struct CompSparse(pub u8);

#[derive(Component, Clone, Debug)]
pub struct Marker;

impl<'a> Arbitrary<'a> for Marker {
    fn arbitrary(_u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Marker)
    }
    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

pub fn check_world_invariants(world: &mut World, shadow: &[Entity]) {
    assert!(
        world.entities().count_spawned() as usize >= shadow.len(),
        "World has fewer spawned entities than tracked: world={}, tracked={}",
        world.entities().count_spawned(),
        shadow.len(),
    );
    for &e in shadow {
        assert!(
            world.entities().contains_spawned(e),
            "Tracked entity {e:?} is not spawned in world"
        );
    }
}
