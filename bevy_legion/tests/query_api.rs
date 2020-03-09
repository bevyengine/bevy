#![allow(clippy::map_clone)]

use legion::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "par-iter")]
use std::sync::atomic::{AtomicUsize, Ordering};

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
fn query_read_entity_data() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = Read::<Pos>::query();

    let mut count = 0;
    for (entity, pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        count += 1;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_try_read_entity_data() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();
    world.insert((), Some((Pos(1., 2., 3.),)));
    world.insert((), Some((Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6))));

    let query = TryRead::<Rot>::query();
    let rots = query
        .iter(&world)
        .map(|x| x.map(|x| *x))
        .collect::<Vec<_>>();
    assert_eq!(rots.iter().filter(|x| x.is_none()).count(), 1);
    assert_eq!(
        rots.iter().cloned().filter_map(|x| x).collect::<Vec<_>>(),
        &[Rot(0.4, 0.5, 0.6)]
    );
}

#[test]
fn query_try_write_entity_data() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();
    world.insert((), Some((Pos(1., 2., 3.),)));
    let entity = world.insert((), Some((Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6))))[0];

    let query = TryWrite::<Rot>::query();
    for mut x in query.iter_mut(&mut world).filter_map(|x| x) {
        *x = Rot(9.0, 9.0, 9.0);
    }
    assert_eq!(
        world.get_component::<Rot>(entity).map(|x| *x),
        Some(Rot(9.0, 9.0, 9.0))
    );
}

#[test]
fn query_cached_read_entity_data() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = Read::<Pos>::query(); //.cached();

    let mut count = 0;
    for (entity, pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        count += 1;
    }

    assert_eq!(components.len(), count);
}

#[test]
#[cfg(feature = "par-iter")]
fn query_read_entity_data_par() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let count = AtomicUsize::new(0);
    let query = Read::<Pos>::query();
    query.par_for_each_chunk_mut(&mut world, |mut chunk| {
        for (entity, pos) in chunk.iter_entities_mut() {
            assert_eq!(expected.get(&entity).unwrap().0, *pos);
            count.fetch_add(1, Ordering::SeqCst);
        }
    });

    assert_eq!(components.len(), count.load(Ordering::SeqCst));
}

#[test]
#[cfg(feature = "par-iter")]
fn query_read_entity_data_par_foreach() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let count = AtomicUsize::new(0);
    let query = Read::<Pos>::query();
    query.par_for_each_mut(&mut world, |_pos| {
        count.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(components.len(), count.load(Ordering::SeqCst));
}

#[test]
fn query_read_entity_data_tuple() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = <(Read<Pos>, Read<Rot>)>::query();

    let mut count = 0;
    for (entity, (pos, rot)) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        assert_eq!(expected.get(&entity).unwrap().1, *rot);
        count += 1;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_write_entity_data() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = Write::<Pos>::query();

    let mut count = 0;
    for (entity, mut pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        count += 1;

        pos.0 = 0.0;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_write_entity_data_tuple() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = <(Write<Pos>, Write<Rot>)>::query();

    let mut count = 0;
    for (entity, (mut pos, mut rot)) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        assert_eq!(expected.get(&entity).unwrap().1, *rot);
        count += 1;

        pos.0 = 0.0;
        rot.0 = 0.0;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_mixed_entity_data_tuple() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = <(Read<Pos>, Write<Rot>)>::query();

    let mut count = 0;
    for (entity, (pos, mut rot)) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        assert_eq!(expected.get(&entity).unwrap().1, *rot);
        count += 1;

        rot.0 = 0.0;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_partial_match() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = <(Read<Pos>, Write<Rot>)>::query();

    let mut count = 0;
    for (entity, (pos, mut rot)) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        assert_eq!(expected.get(&entity).unwrap().1, *rot);
        count += 1;

        rot.0 = 0.0;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_read_shared_data() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    world.insert(shared, components.clone());

    let query = Tagged::<Static>::query();

    let mut count = 0;
    for marker in query.iter(&world) {
        assert_eq!(Static, *marker);
        count += 1;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_on_changed_first() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = Read::<Pos>::query().filter(changed::<Pos>() | changed::<Rot>());

    let mut count = 0;
    for (entity, pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        count += 1;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_on_changed_no_changes() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = Read::<Pos>::query().filter(changed::<Pos>());

    let mut count = 0;
    for (entity, pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        count += 1;
    }

    assert_eq!(components.len(), count);

    count = 0;
    for (entity, pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        count += 1;
    }

    assert_eq!(0, count);
}

#[test]
fn query_on_changed_self_changes() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    let shared = (Static, Model(5));
    let components = vec![
        (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
        (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
    ];

    let mut expected = HashMap::<Entity, (Pos, Rot)>::new();

    for (i, e) in world.insert(shared, components.clone()).iter().enumerate() {
        if let Some((pos, rot)) = components.get(i) {
            expected.insert(*e, (*pos, *rot));
        }
    }

    let query = Write::<Pos>::query().filter(changed::<Pos>());

    let mut count = 0;
    for (entity, mut pos) in query.iter_entities_mut(&mut world) {
        assert_eq!(expected.get(&entity).unwrap().0, *pos);
        *pos = Pos(1., 1., 1.);
        count += 1;
    }

    assert_eq!(components.len(), count);

    count = 0;
    for pos in query.iter_mut(&mut world) {
        assert_eq!(Pos(1., 1., 1.), *pos);
        count += 1;
    }

    assert_eq!(components.len(), count);
}

#[test]
fn query_try_with_changed_filter() {
    let _ = tracing_subscriber::fmt::try_init();

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Sum(f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct A(f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct B(f32);

    let universe = Universe::new();
    let mut world = universe.create_world();

    let sum_entity = world.insert((), Some((Sum(0.),)))[0];
    let a_entity = world.insert((), Some((Sum(0.), A(1.))))[0];
    let b_entity = world.insert((), Some((Sum(0.), B(2.))))[0];
    let a_b_entity = world.insert((), Some((Sum(0.), A(1.), B(2.))))[0];

    let query =
        <(Write<Sum>, TryRead<A>, TryRead<B>)>::query().filter(changed::<A>() | changed::<B>());

    let mut count = 0;
    for (mut sum, a, b) in query.iter_mut(&mut world) {
        sum.0 = a.map_or(0., |x| x.0) + b.map_or(0., |x| x.0);
        count += 1;
    }
    assert_eq!(3, count);
    assert_eq!(
        world.get_component::<Sum>(sum_entity).map(|x| *x),
        Some(Sum(0.))
    );
    assert_eq!(
        world.get_component::<Sum>(a_entity).map(|x| *x),
        Some(Sum(1.))
    );
    assert_eq!(
        world.get_component::<Sum>(b_entity).map(|x| *x),
        Some(Sum(2.))
    );
    assert_eq!(
        world.get_component::<Sum>(a_b_entity).map(|x| *x),
        Some(Sum(3.))
    );

    count = 0;
    for (mut sum, a, b) in query.iter_mut(&mut world) {
        sum.0 = a.map_or(0., |x| x.0) + b.map_or(0., |x| x.0);
        count += 1;
    }
    assert_eq!(0, count);

    *world.get_component_mut::<B>(a_b_entity).unwrap() = B(3.0);
    count = 0;
    for (mut sum, a, b) in query.iter_mut(&mut world) {
        sum.0 = a.map_or(0., |x| x.0) + b.map_or(0., |x| x.0);
        count += 1;
    }
    assert_eq!(1, count);
    assert_eq!(
        world.get_component::<Sum>(a_b_entity).map(|x| *x),
        Some(Sum(4.))
    );
}

#[test]
fn query_iter_chunks_tag() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    world.insert((Static, Model(0)), vec![()]);
    world.insert((Static, Model(1)), vec![()]);
    world.insert((Static, Model(2)), vec![()]);

    let query = <(Tagged<Static>, Tagged<Model>)>::query();

    for chunk in query.iter_chunks(&world) {
        let model = chunk.tag::<Model>().cloned();
        for entity in chunk.entities() {
            assert_eq!(world.get_tag::<Model>(*entity), model.as_ref());
        }
    }
}

#[test]
fn query_iter_tag() {
    let _ = tracing_subscriber::fmt::try_init();

    let universe = Universe::new();
    let mut world = universe.create_world();

    world.insert((Static, Model(0)), vec![(0u32,)]);
    world.insert((Static, Model(1)), vec![(1u32,)]);
    world.insert((Static, Model(2)), vec![(2u32,)]);

    let query = <(Tagged<Static>, Tagged<Model>, Read<u32>)>::query();

    for (s, m, c) in query.iter(&world) {
        assert_eq!(&Static, s);
        assert_eq!(&Model(*c), m);
    }
}
