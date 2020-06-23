use legion::prelude::*;
use legion::storage::ComponentTypeId;
use legion::storage::TagTypeId;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos(f32, f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rot(f32, f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Scale(f32, f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vel(f32, f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Accel(f32, f32, f32);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct Model(u32);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct Static;

#[test]
fn insert() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (1usize, 2f32, 3u16);
    let components = vec![(4f32, 5u64, 6u16), (4f32, 5u64, 6u16)];
    let entities = world.insert(shared, components);

    assert_eq!(2, entities.len());
}

#[test]
fn get_component() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut entities: Vec<Entity> = Vec::new();
    for e in world.insert(shared, components.clone()) {
        entities.push(*e);
    }

    for (i, e) in entities.iter().enumerate() {
        match world.get_component(*e) {
            Some(x) => assert_eq!(components.get(i).map(|(x, _)| x), Some(&x as &Pos)),
            None => assert_eq!(components.get(i).map(|(x, _)| x), None),
        }
        match world.get_component(*e) {
            Some(x) => assert_eq!(components.get(i).map(|(_, x)| x), Some(&x as &Rot)),
            None => assert_eq!(components.get(i).map(|(_, x)| x), None),
        }
    }
}

#[test]
fn get_component_wrong_type() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let entity = *world.insert((), vec![(0f64,)]).get(0).unwrap();

    assert!(world.get_component::<i32>(entity).is_none());
}

#[test]
fn get_shared() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut entities: Vec<Entity> = Vec::new();
    for e in world.insert(shared, components) {
        entities.push(*e);
    }

    for e in entities.iter() {
        assert_eq!(Some(&Static), world.get_tag(*e));
        assert_eq!(Some(&Model(5)), world.get_tag(*e));
    }
}

#[test]
fn get_shared_wrong_type() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let entity = *world.insert((Static,), vec![(0f64,)]).get(0).unwrap();

    assert!(world.get_tag::<Model>(entity).is_none());
}

#[test]
fn delete() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut entities: Vec<Entity> = Vec::new();
    for e in world.insert(shared, components) {
        entities.push(*e);
    }

    for e in entities.iter() {
        assert_eq!(true, world.is_alive(*e));
    }

    for e in entities.iter() {
        world.delete(*e);
        assert_eq!(false, world.is_alive(*e));
    }
}

#[test]
fn delete_all() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut entities: Vec<Entity> = Vec::new();
    for e in world.insert(shared, components) {
        entities.push(*e);
    }

    // Check that the entity allocator knows about the entities
    for e in entities.iter() {
        assert_eq!(true, world.is_alive(*e));
    }

    // Check that the entities are in storage
    let query = <(Read<Pos>, Read<Rot>)>::query();
    assert_eq!(2, query.iter(&world).count());

    world.delete_all();

    // Check that the entity allocator no longer knows about the entities
    for e in entities.iter() {
        assert_eq!(false, world.is_alive(*e));
    }

    // Check that the entities are removed from storage
    let query = <(Read<Pos>, Read<Rot>)>::query();
    assert_eq!(0, query.iter(&world).count());
}

#[test]
fn delete_last() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut entities: Vec<Entity> = Vec::new();
    for e in world.insert(shared, components.clone()) {
        entities.push(*e);
    }

    let last = *entities.last().unwrap();
    world.delete(last);
    assert_eq!(false, world.is_alive(last));

    for (i, e) in entities.iter().take(entities.len() - 1).enumerate() {
        assert_eq!(true, world.is_alive(*e));
        match world.get_component(*e) {
            Some(x) => assert_eq!(components.get(i).map(|(x, _)| x), Some(&x as &Pos)),
            None => assert_eq!(components.get(i).map(|(x, _)| x), None),
        }
        match world.get_component(*e) {
            Some(x) => assert_eq!(components.get(i).map(|(_, x)| x), Some(&x as &Rot)),
            None => assert_eq!(components.get(i).map(|(_, x)| x), None),
        }
    }
}

#[test]
fn delete_first() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut entities: Vec<Entity> = Vec::new();
    for e in world.insert(shared, components.clone()) {
        entities.push(*e);
    }

    let first = *entities.first().unwrap();

    world.delete(first);
    assert_eq!(false, world.is_alive(first));

    for (i, e) in entities.iter().skip(1).enumerate() {
        assert_eq!(true, world.is_alive(*e));
        match world.get_component(*e) {
            Some(x) => assert_eq!(components.get(i + 1).map(|(x, _)| x), Some(&x as &Pos)),
            None => assert_eq!(components.get(i + 1).map(|(x, _)| x), None),
        }
        match world.get_component(*e) {
            Some(x) => assert_eq!(components.get(i + 1).map(|(_, x)| x), Some(&x as &Rot)),
            None => assert_eq!(components.get(i + 1).map(|(_, x)| x), None),
        }
    }
}

#[test]
fn move_from() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world_1 = universe.create_world();
    let mut world_2 = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut world_1_entities: Vec<Entity> = Vec::new();
    for e in world_1.insert(shared, components.clone()) {
        world_1_entities.push(*e);
    }

    let mut world_2_entities: Vec<Entity> = Vec::new();
    for e in world_2.insert(shared, components.clone()) {
        world_2_entities.push(*e);
    }

    world_1.move_from(world_2);

    for (i, e) in world_2_entities.iter().enumerate() {
        assert!(world_1.is_alive(*e));

        let (pos, rot) = components.get(i).unwrap();
        assert_eq!(pos, &world_1.get_component(*e).unwrap() as &Pos);
        assert_eq!(rot, &world_1.get_component(*e).unwrap() as &Rot);
    }
}

#[test]
fn mutate_add_component() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let entities = world.insert(shared, components).to_vec();

    let query_without_scale = <(Read<Pos>, Read<Rot>)>::query();
    let query_with_scale = <(Read<Pos>, Read<Rot>, Read<Scale>)>::query();

    assert_eq!(3, query_without_scale.iter(&world).count());
    assert_eq!(0, query_with_scale.iter(&world).count());

    world
        .add_component(*entities.get(1).unwrap(), Scale(0.5, 0.5, 0.5))
        .unwrap();

    assert_eq!(3, query_without_scale.iter(&world).count());
    assert_eq!(1, query_with_scale.iter(&world).count());
}

#[test]
fn mutate_remove_component() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let entities = world.insert(shared, components).to_vec();

    let query_without_rot = Read::<Pos>::query().filter(!component::<Rot>());
    let query_with_rot = <(Read<Pos>, Read<Rot>)>::query();

    assert_eq!(0, query_without_rot.iter(&world).count());
    assert_eq!(3, query_with_rot.iter(&world).count());

    world
        .remove_component::<Rot>(*entities.get(1).unwrap())
        .unwrap();

    assert_eq!(1, query_without_rot.iter(&world).count());
    assert_eq!(2, query_with_rot.iter(&world).count());
}

#[test]
fn mutate_add_tag() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5),);
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let entities = world.insert(shared, components).to_vec();

    let query_without_static = <(Read<Pos>, Read<Rot>)>::query();
    let query_with_static = <(Read<Pos>, Read<Rot>, Tagged<Static>)>::query();

    assert_eq!(3, query_without_static.iter(&world).count());
    assert_eq!(0, query_with_static.iter(&world).count());

    world.add_tag(*entities.get(1).unwrap(), Static).unwrap();

    assert_eq!(3, query_without_static.iter(&world).count());
    assert_eq!(1, query_with_static.iter(&world).count());
}

#[test]
fn mutate_remove_tag() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5), Static);
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let entities = world.insert(shared, components).to_vec();

    let query_without_static = <(Read<Pos>, Read<Rot>)>::query().filter(!tag::<Static>());
    let query_with_static = <(Read<Pos>, Read<Rot>, Tagged<Static>)>::query();

    assert_eq!(0, query_without_static.iter(&world).count());
    assert_eq!(3, query_with_static.iter(&world).count());

    world
        .remove_tag::<Static>(*entities.get(1).unwrap())
        .unwrap();

    assert_eq!(1, query_without_static.iter(&world).count());
    assert_eq!(2, query_with_static.iter(&world).count());
}

#[test]
fn mutate_change_tag_minimum_test() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5),);
    let components = vec![(Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3))];

    let entities = world.insert(shared, components).to_vec();

    tracing::trace!("STARTING CHANGE");
    world.add_tag(entities[0], Model(3)).unwrap();
    tracing::trace!("CHANGED\n");

    assert_eq!(*world.get_tag::<Model>(entities[0]).unwrap(), Model(3));
}

#[test]
fn delete_entities_on_drop() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let (tx, rx) = crossbeam_channel::unbounded::<legion::event::Event>();

    let shared = (Model(5),);
    let components = vec![(Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3))];

    // Insert the data and store resulting entities in a HashSet
    let mut entities = HashSet::new();
    for entity in world.insert(shared, components) {
        entities.insert(*entity);
    }

    world.subscribe(tx, legion::filter::filter_fns::any());

    //ManuallyDrop::drop(&mut world);
    std::mem::drop(world);

    for e in rx.try_recv() {
        match e {
            legion::event::Event::EntityRemoved(entity, _chunk_id) => {
                assert!(entities.remove(&entity));
            }
            _ => {}
        }
    }

    // Verify that no extra entities are included
    assert!(entities.is_empty());
}

#[test]
#[allow(clippy::suspicious_map)]
fn mutate_change_tag() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5),);
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let entities = world.insert(shared, components).to_vec();

    let query_model_3 = <(Read<Pos>, Read<Rot>)>::query().filter(tag_value(&Model(3)));
    let query_model_5 = <(Read<Pos>, Read<Rot>)>::query().filter(tag_value(&Model(5)));

    assert_eq!(3, query_model_5.iter(&world).count());
    assert_eq!(0, query_model_3.iter(&world).count());

    tracing::trace!("STARTING CHANGE");
    world.add_tag(*entities.get(1).unwrap(), Model(3)).unwrap();
    tracing::trace!("CHANGED\n");

    assert_eq!(
        1,
        query_model_3
            .iter_entities_mut(&mut world)
            .map(|e| {
                tracing::trace!("iter: {:?}", e);
                e
            })
            .count()
    );
    assert_eq!(
        *world.get_tag::<Model>(*entities.get(1).unwrap()).unwrap(),
        Model(3)
    );

    assert_eq!(2, query_model_5.iter(&world).count());
}

// This test repeatedly creates a world with new entities and drops it, reproducing
// https://github.com/TomGillen/legion/issues/92
#[test]
fn lots_of_deletes() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();

    for _ in 0..10000 {
        let shared = (Model(5),);
        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let mut world = universe.create_world();
        world.insert(shared, components).to_vec();
    }
}

#[test]
fn iter_entities() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5),);
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    // Insert the data and store resulting entities in a HashSet
    let mut entities = HashSet::new();
    for entity in world.insert(shared, components) {
        entities.insert(*entity);
    }

    // Verify that all entities in iter_entities() are included
    for entity in world.iter_entities() {
        assert!(entities.remove(&entity));
    }

    // Verify that no extra entities are included
    assert!(entities.is_empty());
}

// Test that World::entity_component_types returns the correct ComponentTypeId
#[test]
fn entity_component_types() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5),);
    let components = vec![(Pos(1., 2., 3.),)];

    let entity = world.insert(shared, components)[0];

    let component_types = world.entity_component_types(entity).unwrap();
    let component_type_ids: Vec<ComponentTypeId> =
        component_types.iter().map(|(id, _)| *id).collect();
    assert_eq!(1, component_type_ids.len());
    assert!(component_type_ids.contains(&ComponentTypeId::of::<Pos>()));
}

// Test that World::entity_tag_types returns the correct TagTypeId
#[test]
fn entity_tag_types() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Model(5),);
    let components = vec![(Pos(1., 2., 3.),)];

    let entity = world.insert(shared, components)[0];

    let tag_types = world.entity_tag_types(entity).unwrap();
    let tag_type_ids: Vec<TagTypeId> = tag_types.iter().map(|(id, _)| *id).collect();
    assert_eq!(1, tag_type_ids.len());
    assert!(tag_type_ids.contains(&TagTypeId::of::<Model>()));
}
