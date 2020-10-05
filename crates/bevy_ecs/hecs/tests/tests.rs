// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// modified by Bevy contributors

use bevy_hecs::*;

#[test]
fn random_access() {
    let mut world = World::new();
    let e = world.spawn(("abc", 123));
    let f = world.spawn(("def", 456, true));
    assert_eq!(*world.get::<&str>(e).unwrap(), "abc");
    assert_eq!(*world.get::<i32>(e).unwrap(), 123);
    assert_eq!(*world.get::<&str>(f).unwrap(), "def");
    assert_eq!(*world.get::<i32>(f).unwrap(), 456);
    *world.get_mut::<i32>(f).unwrap() = 42;
    assert_eq!(*world.get::<i32>(f).unwrap(), 42);
}

#[test]
fn despawn() {
    let mut world = World::new();
    let e = world.spawn(("abc", 123));
    let f = world.spawn(("def", 456));
    assert_eq!(world.query::<()>().iter().count(), 2);
    world.despawn(e).unwrap();
    assert_eq!(world.query::<()>().iter().count(), 1);
    assert!(world.get::<&str>(e).is_err());
    assert!(world.get::<i32>(e).is_err());
    assert_eq!(*world.get::<&str>(f).unwrap(), "def");
    assert_eq!(*world.get::<i32>(f).unwrap(), 456);
}

#[test]
fn query_all() {
    let mut world = World::new();
    let e = world.spawn(("abc", 123));
    let f = world.spawn(("def", 456));

    let ents = world
        .query::<(Entity, &i32, &&str)>()
        .iter()
        .map(|(e, &i, &s)| (e, i, s))
        .collect::<Vec<_>>();
    assert_eq!(ents.len(), 2);
    assert!(ents.contains(&(e, 123, "abc")));
    assert!(ents.contains(&(f, 456, "def")));

    let ents = world.query::<Entity>().iter().collect::<Vec<_>>();
    assert_eq!(ents.len(), 2);
    assert!(ents.contains(&e));
    assert!(ents.contains(&f));
}

#[test]
fn query_single_component() {
    let mut world = World::new();
    let e = world.spawn(("abc", 123));
    let f = world.spawn(("def", 456, true));
    let ents = world
        .query::<(Entity, &i32)>()
        .iter()
        .map(|(e, &i)| (e, i))
        .collect::<Vec<_>>();
    assert_eq!(ents.len(), 2);
    assert!(ents.contains(&(e, 123)));
    assert!(ents.contains(&(f, 456)));
}

#[test]
fn query_missing_component() {
    let mut world = World::new();
    world.spawn(("abc", 123));
    world.spawn(("def", 456));
    assert!(world.query::<(&bool, &i32)>().iter().next().is_none());
}

#[test]
fn query_sparse_component() {
    let mut world = World::new();
    world.spawn(("abc", 123));
    let f = world.spawn(("def", 456, true));
    let ents = world
        .query::<(Entity, &bool)>()
        .iter()
        .map(|(e, &b)| (e, b))
        .collect::<Vec<_>>();
    assert_eq!(ents, &[(f, true)]);
}

#[test]
fn query_optional_component() {
    let mut world = World::new();
    let e = world.spawn(("abc", 123));
    let f = world.spawn(("def", 456, true));
    let ents = world
        .query::<(Entity, Option<&bool>, &i32)>()
        .iter()
        .map(|(e, b, &i)| (e, b.copied(), i))
        .collect::<Vec<_>>();
    assert_eq!(ents.len(), 2);
    assert!(ents.contains(&(e, None, 123)));
    assert!(ents.contains(&(f, Some(true), 456)));
}

#[test]
fn build_entity() {
    let mut world = World::new();
    let mut entity = EntityBuilder::new();
    entity.add("abc");
    entity.add(123);
    let e = world.spawn(entity.build());
    entity.add("def");
    entity.add([0u8; 1024]);
    entity.add(456);
    let f = world.spawn(entity.build());
    assert_eq!(*world.get::<&str>(e).unwrap(), "abc");
    assert_eq!(*world.get::<i32>(e).unwrap(), 123);
    assert_eq!(*world.get::<&str>(f).unwrap(), "def");
    assert_eq!(*world.get::<i32>(f).unwrap(), 456);
}

#[test]
fn dynamic_components() {
    let mut world = World::new();
    let e = world.spawn((42,));
    world.insert(e, (true, "abc")).unwrap();
    assert_eq!(
        world
            .query::<(Entity, &i32, &bool)>()
            .iter()
            .map(|(e, &i, &b)| (e, i, b))
            .collect::<Vec<_>>(),
        &[(e, 42, true)]
    );
    assert_eq!(world.remove_one::<i32>(e), Ok(42));
    assert_eq!(
        world
            .query::<(Entity, &i32, &bool)>()
            .iter()
            .map(|(e, &i, &b)| (e, i, b))
            .collect::<Vec<_>>(),
        &[]
    );
    assert_eq!(
        world
            .query::<(Entity, &bool, &&str)>()
            .iter()
            .map(|(e, &b, &s)| (e, b, s))
            .collect::<Vec<_>>(),
        &[(e, true, "abc")]
    );
}

#[test]
fn shared_borrow() {
    let mut world = World::new();
    world.spawn(("abc", 123));
    world.spawn(("def", 456));

    world.query::<(&i32, &i32)>();
}

#[test]
#[cfg(feature = "macros")]
fn derived_bundle() {
    #[derive(Bundle)]
    struct Foo {
        x: i32,
        y: f64,
    }

    let mut world = World::new();
    let e = world.spawn(Foo { x: 42, y: 1.0 });
    assert_eq!(*world.get::<i32>(e).unwrap(), 42);
    assert_eq!(*world.get::<f64>(e).unwrap(), 1.0);
}

#[test]
#[cfg(feature = "macros")]
#[should_panic(expected = "each type must occur at most once")]
fn bad_bundle_derive() {
    #[derive(Bundle)]
    struct Foo {
        x: i32,
        y: i32,
    }

    let mut world = World::new();
    world.spawn(Foo { x: 42, y: 42 });
}

#[test]
#[cfg_attr(miri, ignore)]
fn spawn_many() {
    let mut world = World::new();
    const N: usize = 100_000;
    for _ in 0..N {
        world.spawn((42u128,));
    }
    assert_eq!(world.iter().count(), N);
}

#[test]
fn clear() {
    let mut world = World::new();
    world.spawn(("abc", 123));
    world.spawn(("def", 456, true));
    world.clear();
    assert_eq!(world.iter().count(), 0);
}

#[test]
#[should_panic(expected = "twice on the same borrow")]
fn alias() {
    let mut world = World::new();
    world.spawn(("abc", 123));
    world.spawn(("def", 456, true));
    let mut q = world.query_mut::<Entity>();
    let _a = q.iter().collect::<Vec<_>>();
    let _b = q.iter().collect::<Vec<_>>();
}

#[test]
fn remove_missing() {
    let mut world = World::new();
    let e = world.spawn(("abc", 123));
    assert!(world.remove_one::<bool>(e).is_err());
}

#[test]
fn query_batched() {
    let mut world = World::new();
    let a = world.spawn(());
    let b = world.spawn(());
    let c = world.spawn((42,));
    assert_eq!(world.query::<()>().iter_batched(1).count(), 3);
    assert_eq!(world.query::<()>().iter_batched(2).count(), 2);
    assert_eq!(
        world.query::<()>().iter_batched(2).flat_map(|x| x).count(),
        3
    );
    // different archetypes are always in different batches
    assert_eq!(world.query::<()>().iter_batched(3).count(), 2);
    assert_eq!(
        world.query::<()>().iter_batched(3).flat_map(|x| x).count(),
        3
    );
    assert_eq!(world.query::<()>().iter_batched(4).count(), 2);
    let entities = world
        .query::<Entity>()
        .iter_batched(1)
        .flat_map(|x| x)
        .map(|e| e)
        .collect::<Vec<_>>();
    dbg!(&entities);
    assert_eq!(entities.len(), 3);
    assert!(entities.contains(&a));
    assert!(entities.contains(&b));
    assert!(entities.contains(&c));
}

#[test]
fn spawn_batch() {
    let mut world = World::new();
    world.spawn_batch((0..100).map(|x| (x, "abc")));
    let entities = world.query::<&i32>().iter().map(|&x| x).collect::<Vec<_>>();
    assert_eq!(entities.len(), 100);
}

#[test]
fn query_one() {
    let mut world = World::new();
    let a = world.spawn(("abc", 123));
    let b = world.spawn(("def", 456));
    let c = world.spawn(("ghi", 789, true));
    assert_eq!(world.query_one::<&i32>(a).unwrap().get(), Some(&123));
    assert_eq!(world.query_one::<&i32>(b).unwrap().get(), Some(&456));
    assert!(world.query_one::<(&i32, &bool)>(a).unwrap().get().is_none());
    assert_eq!(
        world.query_one::<(&i32, &bool)>(c).unwrap().get(),
        Some((&789, &true))
    );
    world.despawn(a).unwrap();
    assert!(world.query_one::<&i32>(a).is_err());
}

#[test]
fn remove_tracking() {
    let mut world = World::new();
    let a = world.spawn(("abc", 123));
    let b = world.spawn(("abc", 123));

    world.despawn(a).unwrap();
    assert_eq!(
        world.removed::<i32>(),
        &[a],
        "despawning results in 'removed component' state"
    );
    assert_eq!(
        world.removed::<&'static str>(),
        &[a],
        "despawning results in 'removed component' state"
    );

    world.insert_one(b, 10.0).unwrap();
    assert_eq!(
        world.removed::<i32>(),
        &[a],
        "archetype moves does not result in 'removed component' state"
    );

    world.remove_one::<i32>(b).unwrap();
    assert_eq!(
        world.removed::<i32>(),
        &[a, b],
        "removing a component results in a 'removed component' state"
    );

    world.clear_trackers();
    assert_eq!(
        world.removed::<i32>(),
        &[],
        "clearning trackers clears removals"
    );
    assert_eq!(
        world.removed::<&'static str>(),
        &[],
        "clearning trackers clears removals"
    );
    assert_eq!(
        world.removed::<f64>(),
        &[],
        "clearning trackers clears removals"
    );

    let c = world.spawn(("abc", 123));
    let d = world.spawn(("abc", 123));
    world.clear();
    assert_eq!(
        world.removed::<i32>(),
        &[c, d],
        "world clears result in 'removed component' states"
    );
    assert_eq!(
        world.removed::<&'static str>(),
        &[c, d, b],
        "world clears result in 'removed component' states"
    );
    assert_eq!(
        world.removed::<f64>(),
        &[b],
        "world clears result in 'removed component' states"
    );
}

#[test]
fn added_tracking() {
    let mut world = World::new();
    let a = world.spawn((123,));

    assert_eq!(world.query::<&i32>().iter().count(), 1);
    assert_eq!(world.query::<Added<i32>>().iter().count(), 1);
    assert_eq!(world.query_mut::<&i32>().iter().count(), 1);
    assert_eq!(world.query_mut::<Added<i32>>().iter().count(), 1);
    assert!(world.query_one::<&i32>(a).unwrap().get().is_some());
    assert!(world.query_one::<Added<i32>>(a).unwrap().get().is_some());
    assert!(world.query_one_mut::<&i32>(a).unwrap().get().is_some());
    assert!(world
        .query_one_mut::<Added<i32>>(a)
        .unwrap()
        .get()
        .is_some());

    world.clear_trackers();

    assert_eq!(world.query::<&i32>().iter().count(), 1);
    assert_eq!(world.query::<Added<i32>>().iter().count(), 0);
    assert_eq!(world.query_mut::<&i32>().iter().count(), 1);
    assert_eq!(world.query_mut::<Added<i32>>().iter().count(), 0);
    assert!(world.query_one_mut::<&i32>(a).unwrap().get().is_some());
    assert!(world
        .query_one_mut::<Added<i32>>(a)
        .unwrap()
        .get()
        .is_none());
}
