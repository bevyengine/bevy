// FIXME(Relationships) add a .len() method to `RelationAccess` and `RelationAccessMut` maybe also implement ExactSizeIterator?

use std::convert::TryInto;

use crate::{
    component::{ComponentDescriptor, StorageType, TargetType},
    query::TargetFilter,
};
use crate::{prelude::*, query::RelationAccess};

#[test]
fn relation_spawn() {
    relation_spawn_raw(StorageType::Table);
    relation_spawn_raw(StorageType::SparseSet);
}
#[allow(clippy::bool_assert_comparison)]
fn relation_spawn_raw(storage_type: StorageType) {
    let mut world = World::new();

    world
        .register_component(ComponentDescriptor::new_targeted::<ChildOf>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    struct ChildOf;

    let parent = world.spawn().id();
    let not_parent = world.spawn().id();

    let mut child = world.spawn();
    let child = child.insert_relation(ChildOf, parent);

    assert!(child.contains_relation::<ChildOf>(parent));
    assert_eq!(child.contains_relation::<ChildOf>(not_parent), false);
    assert_eq!(child.contains_relation::<u32>(parent), false);

    assert!(child.remove_relation::<ChildOf>(parent).is_some());
    assert!(child.remove_relation::<ChildOf>(parent).is_none());
    assert!(child.remove_relation::<u32>(parent).is_none());
    assert!(child.remove_relation::<ChildOf>(not_parent).is_none());
}

#[test]
fn relation_query() {
    relation_query_raw(StorageType::Table);
    relation_query_raw(StorageType::SparseSet);
}
fn relation_query_raw(storage_type: StorageType) {
    struct ChildOf;

    let mut world = World::new();

    world
        .register_component(ComponentDescriptor::new_targeted::<ChildOf>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    let parent1 = world.spawn().id();
    let child1 = world.spawn().insert_relation(ChildOf, parent1).id();
    let parent2 = world.spawn().id();
    let child2 = world.spawn().insert_relation(ChildOf, parent2).id();

    let mut query = world.query::<(Entity, &Relation<ChildOf>)>();
    let mut iter = query.iter_mut(&mut world);
    assert!(iter.next().unwrap().0 == child1);
    assert!(iter.next().unwrap().0 == child2);
    assert!(matches!(iter.next(), None));

    query
        .new_target_filters(&world, TargetFilter::<ChildOf>::new().target(parent1))
        .apply_filters();
    let mut iter = query.iter_mut(&mut world);
    assert!(iter.next().unwrap().0 == child1);
    assert!(matches!(iter.next(), None));

    query
        .new_target_filters(&world, TargetFilter::<ChildOf>::new().target(parent2))
        .apply_filters();
    let mut iter = query.iter_mut(&mut world);
    assert!(iter.next().unwrap().0 == child2);
    assert!(matches!(iter.next(), None));

    query
        .new_target_filters(
            &world,
            TargetFilter::<ChildOf>::new()
                .target(parent1)
                .target(parent2),
        )
        .apply_filters();
    let mut iter = query.iter_mut(&mut world);
    assert!(matches!(iter.next(), None));
}

#[test]
fn relation_access() {
    relation_access_raw(StorageType::Table);
    relation_access_raw(StorageType::SparseSet);
}
fn relation_access_raw(storage_type: StorageType) {
    #[derive(Debug, PartialEq, Eq)]
    struct ChildOf {
        despawn_recursive: bool,
    }
    let mut world = World::new();

    world
        .register_component(ComponentDescriptor::new_targeted::<ChildOf>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    let random_parent = world.spawn().id();
    let parent1 = world.spawn().id();
    let parent2 = world.spawn().id();
    let child1 = world
        .spawn()
        .insert_relation(
            ChildOf {
                despawn_recursive: true,
            },
            parent1,
        )
        .insert_relation(
            ChildOf {
                despawn_recursive: false,
            },
            random_parent,
        )
        .id();
    let child2 = world
        .spawn()
        .insert_relation(
            ChildOf {
                despawn_recursive: false,
            },
            parent2,
        )
        .insert_relation(
            ChildOf {
                despawn_recursive: true,
            },
            random_parent,
        )
        .id();

    let mut query = world.query::<(Entity, &Relation<ChildOf>)>();

    query
        .new_target_filters(&world, TargetFilter::<ChildOf>::new().target(parent1))
        .apply_filters();
    let mut iter = query.iter(&world);
    let (child, mut accessor) = iter.next().unwrap();
    assert!(child == child1);
    assert_eq!(
        accessor.next().unwrap(),
        (
            parent1,
            &ChildOf {
                despawn_recursive: true
            }
        )
    );
    assert!(matches!(accessor.next(), None));
    assert!(matches!(iter.next(), None));

    query
        .new_target_filters(&world, TargetFilter::<ChildOf>::new().target(parent2))
        .apply_filters();
    let mut iter = query.iter(&world);
    let (child, mut accessor) = iter.next().unwrap();
    assert!(child == child2);
    assert_eq!(
        accessor.next().unwrap(),
        (
            parent2,
            &ChildOf {
                despawn_recursive: false
            }
        )
    );
    assert!(matches!(accessor.next(), None));
    assert!(matches!(iter.next(), None));

    query.clear_target_filters(&world);
    let mut iter = query.iter(&world);
    //
    let (child, mut accessor) = iter.next().unwrap();
    assert!(child == child1);
    assert_eq!(
        accessor.next().unwrap(),
        (
            random_parent,
            &ChildOf {
                despawn_recursive: false
            }
        )
    );
    assert_eq!(
        accessor.next().unwrap(),
        (
            parent1,
            &ChildOf {
                despawn_recursive: true
            }
        )
    );
    assert!(matches!(accessor.next(), None));
    //
    let (child, mut accessor) = iter.next().unwrap();
    assert!(child == child2);
    assert_eq!(
        accessor.next().unwrap(),
        (
            random_parent,
            &ChildOf {
                despawn_recursive: true
            }
        )
    );
    assert_eq!(
        accessor.next().unwrap(),
        (
            parent2,
            &ChildOf {
                despawn_recursive: false
            }
        )
    );
    assert!(matches!(accessor.next(), None));
    assert!(matches!(iter.next(), None));
}

#[test]
fn relation_query_mut() {
    relation_query_mut_raw(StorageType::Table);
    relation_query_mut_raw(StorageType::SparseSet);
}

fn relation_query_mut_raw(storage_type: StorageType) {
    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    struct MyRelation(bool, u32);

    struct Fragment<const N: usize>;

    let mut world = World::new();
    world
        .register_component(ComponentDescriptor::new_targeted::<MyRelation>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    let target1 = world.spawn().insert(Fragment::<1>).id();
    let target2 = world.spawn().insert(Fragment::<1>).id();
    let target3 = world.spawn().id();

    let targeter1 = world
        .spawn()
        .insert(Fragment::<0>)
        .insert("targeter1")
        .insert_relation(MyRelation(true, 10), target1)
        .insert_relation(MyRelation(false, 48), target2)
        .insert_relation(MyRelation(false, 14), target3)
        .id();
    let targeter2 = world
        .spawn()
        .insert("targeter2")
        .insert_relation(MyRelation(false, 75), target1)
        .insert_relation(MyRelation(true, 22), target2)
        .id();
    let targeter3 = world
        .spawn()
        .insert(Fragment::<0>)
        .insert("targeter3")
        .insert_relation(MyRelation(true, 839), target2)
        .insert_relation(MyRelation(true, 3), target3)
        .id();

    let mut query = world.query::<(Entity, &mut Relation<MyRelation>, &&str)>();

    query
        .new_target_filters(&world, TargetFilter::<MyRelation>::new().target(target2))
        .apply_filters();
    for (_, mut accessor, _) in query.iter_mut(&mut world) {
        let (_, mut rel) = accessor.single();
        rel.0 = !rel.0;
        rel.1 += 10;
    }

    query
        .new_target_filters(
            &world,
            TargetFilter::<MyRelation>::new()
                .target(target1)
                .target(target2),
        )
        .apply_filters();
    let mut was_targeter1 = false;
    let mut was_targeter2 = false;
    for (targeter, accessor, name) in query.iter_mut(&mut world) {
        match () {
            _ if targeter == targeter1 => {
                was_targeter1 = true;
                assert_eq!(*name, "targeter1");
                let targets = accessor.map(|(t, rel)| (t, *rel)).collect::<Vec<_>>();
                assert_eq!(&targets[0], &(target1, MyRelation(true, 10)));
                assert_eq!(&targets[1], &(target2, MyRelation(true, 58)));
                assert_eq!(targets.len(), 2);
            }
            _ if targeter == targeter2 => {
                was_targeter2 = true;
                assert_eq!(*name, "targeter2");
                let targets = accessor.map(|(t, rel)| (t, *rel)).collect::<Vec<_>>();
                assert_eq!(&targets[0], &(target1, MyRelation(false, 75)));
                assert_eq!(&targets[1], &(target2, MyRelation(false, 32)));
                assert_eq!(targets.len(), 2);
            }
            _ => panic!(),
        }
    }
    assert!(was_targeter1 && was_targeter2);

    query.clear_target_filters(&world);
    for (_, accessor, _) in query.iter_mut(&mut world) {
        for (_, mut rel) in accessor {
            rel.0 = !rel.0;
            rel.1 *= 2;
        }
    }

    let mut was_targeter1 = false;
    let mut was_targeter2 = false;
    let mut was_targeter3 = false;
    for (targeter, accessor, name) in query.iter_mut(&mut world) {
        match () {
            _ if targeter == targeter1 => {
                was_targeter1 = true;
                assert_eq!(*name, "targeter1");
                let targets = accessor.map(|(t, rel)| (t, *rel)).collect::<Vec<_>>();
                assert_eq!(&targets[0], &(target1, MyRelation(false, 20)));
                assert_eq!(&targets[1], &(target2, MyRelation(false, 116)));
                assert_eq!(&targets[2], &(target3, MyRelation(true, 28)));
                assert_eq!(targets.len(), 3);
            }
            _ if targeter == targeter2 => {
                was_targeter2 = true;
                assert_eq!(*name, "targeter2");
                let targets = accessor.map(|(t, rel)| (t, *rel)).collect::<Vec<_>>();
                assert_eq!(&targets[0], &(target1, MyRelation(true, 150)));
                assert_eq!(&targets[1], &(target2, MyRelation(true, 64)));
                assert_eq!(targets.len(), 2);
            }
            _ if targeter == targeter3 => {
                was_targeter3 = true;
                assert_eq!(*name, "targeter3");
                let targets = accessor.map(|(t, rel)| (t, *rel)).collect::<Vec<_>>();
                assert_eq!(&targets[0], &(target2, MyRelation(true, 849 * 2)));
                assert_eq!(&targets[1], &(target3, MyRelation(false, 6)));
                assert_eq!(targets.len(), 2);
            }
            _ => panic!(),
        }
    }
    assert!(was_targeter1 && was_targeter2 && was_targeter3);
}

#[test]
fn some_example_code() {
    #[derive(PartialEq, Eq, Debug)]
    struct MyRelation;

    let mut world = World::new();

    let target1 = world.spawn().id();
    let target2 = world.spawn().id();
    let my_entity = world
        .spawn()
        .insert_relation(MyRelation, target1)
        .insert_relation(MyRelation, target2)
        .id();

    let mut iterated_entities = Vec::new();
    let mut query = world.query::<(Entity, &Relation<MyRelation>)>();
    for (entity, relations) in query.iter_mut(&mut world) {
        iterated_entities.push(entity);
        assert_eq!(
            &relations.collect::<Vec<_>>(),
            &[(target1, &MyRelation), (target2, &MyRelation)],
        );
    }

    assert_eq!(&iterated_entities, &[my_entity]);
}

macro_rules! self_query_conflict_tests {
    ($($name:ident => <$param:ty>)*) => {
        $(
            #[test]
            #[should_panic]
            fn $name() {
                let mut world = World::new();
                world.query::<$param>();
            }
        )*
    };
}

self_query_conflict_tests!(
    mut_and_mut => <(&mut Relation<u32>, &mut Relation<u32>)>
    mut_and_ref => <(&mut Relation<u32>, &Relation<u32>)>
    ref_and_mut => <(&Relation<u32>, &mut Relation<u32>)>
    rel_and_mut => <(&Relation<u32>, &mut u32)>
    rel_mut_and_ref => <(&mut Relation<u32>, &u32)>
    mut_and_rel => <(&mut u32, &Relation<u32>)>
    ref_and_rel_mut => <(&u32, &mut Relation<u32>)>
    mut_and_rel_mut => <(&mut u32, &mut Relation<u32>)>
    rel_mut_and_mut => <(&mut Relation<u32>, &mut u32)>
);

macro_rules! no_self_query_conflict_tests {
    ($($name:ident => <$param:ty>)*) => {
        $(
            #[test]
            fn $name() {
                let mut world = World::new();
                world.query::<$param>();
            }
        )*
    };
}

no_self_query_conflict_tests!(
    rel_and_rel => <(&Relation<u32>, &Relation<u32>)>
    rel_and_diff_rel => <(&Relation<u32>, &Relation<u64>)>
    rel_mut_and_diff_rel_mut => <(&mut Relation<u32>, &mut Relation<u64>)>
    rel_and_diff_rel_mut => <(&Relation<u32>, &mut Relation<u64>)>
    rel_mut_and_diff_rel => <(&mut Relation<u32>, &Relation<u64>)>
    rel_and_ref => <(&Relation<u32>, &u32)>
    ref_and_rel => <(&u32, &Relation<u32>)>
    rel_mut_and_diff_ref => <(&mut Relation<u32>, &u64)>
    rel_and_diff_mut => <(&Relation<u32>, &mut u64)>
    ref_and_diff_rel_mut => <(&u64, &mut Relation<u32>)>
    mut_and_diff_rel => <(&mut u64, &Relation<u32>)>
    mut_and_diff_rel_mut => <(&mut u64, &mut Relation<u32>)>
    rel_mut_and_diff_mut => <(&mut Relation<u32>, &mut u64)>
);

#[test]
fn compiles() {
    let mut world = World::new();

    let mut query = world.query::<&u32>();

    let borrows = query.iter(&world).collect::<Vec<_>>();
    query.clear_target_filters(&world);
    let _borrows2 = query.iter(&world).collect::<Vec<_>>();
    dbg!(borrows);
}

/**
```compile_fail
use bevy_ecs::prelude::*;

let mut world = World::new();
let mut query = world.query::<&Relation<u32>>();
let _borrows = query.iter(&world).collect::<Vec<_>>();
query.clear_target_filters(&world);
let _borrows2 = query.iter(&world).collect::<Vec<_>>();
drop(_borrows); // If this doesn't fail to compile we have unsoundness - Boxy
```
*/
pub fn _compile_fail() {}

#[test]
fn explicit_path() {
    let mut world = World::new();
    let mut query = world.query::<(&Relation<u32>, &Relation<u32>)>();
    let target = world.spawn().id();

    query
        .new_target_filters::<u32, InData<InTuple<_, 0>>>(
            &world,
            TargetFilter::new().target(target),
        )
        .apply_filters();
}

#[test]
fn foo() {
    let mut world = World::new();
    let mut query = world.query::<&Relation<u32>>();
    let target = world.spawn().id();

    query
        .new_target_filters(&world, TargetFilter::<u32>::new().target(target))
        .apply_filters()
        .for_each(&world, |_| ())
}

#[test]
#[allow(clippy::bool_assert_comparison)]
fn conflict_without_relation() {
    let mut world = World::new();
    let q1 = world.query::<(&mut u32, &Relation<u64>)>();
    let q2 = world.query_filtered::<&mut u32, Without<Relation<u64>>>();
    assert_eq!(
        q1.component_access.is_compatible(&q2.component_access),
        false
    );
}

#[test]
fn without_filter() {
    without_filter_raw(StorageType::Table);
    without_filter_raw(StorageType::SparseSet);
}

fn without_filter_raw(storage_type: StorageType) {
    let mut world = World::new();
    world
        .register_component(ComponentDescriptor::new_targeted::<u32>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    struct MyRelation;

    let target1 = world.spawn().id();
    let target2 = world.spawn().id();
    world
        .spawn()
        .insert(String::from("blah"))
        .insert_relation(MyRelation, target1)
        .id();
    let source2 = world
        .spawn()
        .insert(String::from("OwO"))
        .insert_relation(MyRelation, target2)
        .id();

    let no_relation1 = world.spawn().insert(String::from("hiiii")).id();

    let data = world
        .query_filtered::<(Entity, &String), Without<Relation<MyRelation>>>()
        .iter(&world)
        .collect::<Vec<(Entity, &String)>>();
    assert_eq!(&data, &[(no_relation1, &String::from("hiiii"))]);

    let data = world
        .query_filtered::<(Entity, &String), Without<Relation<MyRelation>>>()
        .new_target_filters(&world, TargetFilter::<MyRelation>::new().target(target1))
        .apply_filters()
        .iter(&world)
        .collect::<Vec<(Entity, &String)>>();
    assert_eq!(
        &data,
        &[
            (no_relation1, &String::from("hiiii")),
            (source2, &String::from("OwO"))
        ]
    );
}

#[test]
fn relations_dont_yield_components() {
    relations_dont_yield_components_raw(StorageType::SparseSet);
    relations_dont_yield_components_raw(StorageType::Table);
}

fn relations_dont_yield_components_raw(storage_type: StorageType) {
    let mut world = World::new();

    world
        .register_component(ComponentDescriptor::new_targeted::<u32>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    let _has_component = world.spawn().insert(10_u32).id();
    let target1 = world.spawn().id();
    let _has_both = world
        .spawn()
        .insert_relation(12_u32, target1)
        .insert(14_u32)
        .id();
    let target2 = world.spawn().id();
    let _has_relation = world.spawn().insert_relation(16_u32, target2).id();

    let mut q = world.query::<&Relation<u32>>();
    let [first, second]: [RelationAccess<_>; 2] =
        q.iter(&world).collect::<Vec<_>>().try_into().unwrap();

    assert_eq!(&first.collect::<Vec<_>>(), &[(target1, &12_u32)]);
    assert_eq!(&second.collect::<Vec<_>>(), &[(target2, &16_u32)]);

    let [first, second]: [&u32; 2] = world
        .query::<&u32>()
        .iter(&world)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    assert_eq!(first, &10_u32);
    assert_eq!(second, &14_u32);
}

#[test]
fn duplicated_target_filters() {
    duplicated_target_filters_raw(StorageType::SparseSet);
    duplicated_target_filters_raw(StorageType::Table);
}

fn duplicated_target_filters_raw(storage_type: StorageType) {
    let mut world = World::new();

    world
        .register_component(ComponentDescriptor::new_targeted::<u32>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    let target = world.spawn().id();
    let _source = world.spawn().insert_relation(10_u32, target).id();

    let mut q = world.query::<&Relation<u32>>();
    let [relations]: [RelationAccess<_>; 1] = q
        .new_target_filters(
            &world,
            TargetFilter::<u32>::new().target(target).target(target),
        )
        .apply_filters()
        .iter(&world)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let [(rel_target, rel_data)]: [(Entity, &u32); 1] =
        relations.collect::<Vec<_>>().try_into().unwrap();
    assert_eq!(rel_target, target);
    assert_eq!(rel_data, &10);
}

#[test]
fn with_filter() {
    with_filter_raw(StorageType::Table);
    with_filter_raw(StorageType::SparseSet);
}

fn with_filter_raw(storage_type: StorageType) {
    let mut world = World::new();
    world
        .register_component(ComponentDescriptor::new_targeted::<u32>(
            storage_type,
            TargetType::Entity,
        ))
        .unwrap();

    let no_relation = world.spawn().insert(10_u32).id();
    let target1 = world.spawn().id();
    let has_relation = world.spawn().insert_relation(12_u32, target1).id();
    let target2 = world.spawn().id();
    let has_both = world
        .spawn()
        .insert_relation(14_u32, target2)
        .insert(16_u32)
        .id();
    let many_relations = world
        .spawn()
        .insert_relation(18_u32, target1)
        .insert_relation(20_u32, target2)
        .id();

    let mut q = world.query_filtered::<Entity, With<Relation<u32>>>();
    let [e1, e2, e3]: [Entity; 3] = q.iter(&world).collect::<Vec<_>>().try_into().unwrap();
    assert_eq!(e1, has_relation);
    assert_eq!(e2, has_both);
    assert_eq!(e3, many_relations);
    let [e1, e2]: [Entity; 2] = q
        .new_target_filters(&world, TargetFilter::<u32>::new().target(target1))
        .apply_filters()
        .iter(&world)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    assert_eq!(e1, has_relation);
    assert_eq!(e2, many_relations);
    let [e1, e2]: [Entity; 2] = q
        .new_target_filters(&world, TargetFilter::<u32>::new().target(target2))
        .apply_filters()
        .iter(&world)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    assert_eq!(e1, has_both);
    assert_eq!(e2, many_relations);
    let []: [Entity; 0] = q
        .new_target_filters(&world, TargetFilter::<u32>::new().target(no_relation))
        .apply_filters()
        .iter(&world)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
}

#[test]
pub fn sparse_set_relation_registration() {
    let mut world = World::new();
    world
        .register_component(ComponentDescriptor::new_targeted::<String>(
            StorageType::SparseSet,
            TargetType::Entity,
        ))
        .unwrap();
    let mut q = world.query::<&Relation<String>>();
    assert!(q.iter(&world).next().is_none());

    let target = world.spawn().id();
    world
        .spawn()
        .insert_relation(String::from("UwowU"), target)
        .id();

    use std::any::TypeId;
    let ty_id = world
        .components
        .component_info(TypeId::of::<String>())
        .unwrap()
        .id();

    assert_eq!(
        world
            .storages
            .sparse_sets
            .get(ty_id, Some(target))
            .unwrap()
            .len(),
        1
    );
}
