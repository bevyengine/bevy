//@no-rustfix

use bevy_ecs::prelude::*;
use bevy_ecs::world::FilteredEntityMut;
fn main() {
    let mut world = World::new();
    let parent = world.spawn_empty().id();
    let _ = world.spawn(ChildOf(parent)).id();

    let children_id = world.register_component::<Children>();

    let mut query = QueryBuilder::<FilteredEntityMut>::new(&mut world)
        .data::<&Children>()
        .build();
    let mut filtered_entity = query.single_mut(&mut world).unwrap();

    let _borrows_r1 = filtered_entity.get_relationship_targets_by_id(children_id);
    let _borrows_r1_mutably = filtered_entity.get_mut_by_id(children_id);
}