use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{Event, EventWriter},
    observer::Trigger,
    world::World,
};
use bevy_hierarchy::{BuildChildren, Children, Parent};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{seq::IteratorRandom, Rng};
use rand_chacha::ChaCha8Rng;

const DENSITY: usize = 20; // percent of nodes with listeners
const ENTITY_DEPTH: usize = 64;
const ENTITY_WIDTH: usize = 200;
const N_EVENTS: usize = 500;
fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

pub fn event_propagation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("event_propagation");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));

    group.bench_function("base", |bencher| {
        let mut world = World::new();
        let approx_invoke_per_iter = (N_EVENTS * (ENTITY_DEPTH + 1)) * DENSITY / 100;
        let mut entities = vec![];
        for _ in 0..approx_invoke_per_iter {
            entities.push(world.spawn_empty().observe(empty_listener_base).id());
        }
        entities.shuffle(&mut deterministic_rand());
        bencher.iter(|| {
            send_base_event(&mut world, &entities);
        });
    });
    group.bench_function("single_event_type", |bencher| {
        let mut world = World::new();
        let (roots, leaves, nodes) = spawn_listener_hierarchy(&mut world);
        assert!(roots.len() + leaves.len() + nodes.len() == (ENTITY_DEPTH + 1) * ENTITY_WIDTH);
        add_listeners_to_hierarchy::<DENSITY, 1>(&roots, &leaves, &nodes, &mut world);

        bencher.iter(|| {
            send_events::<1, N_EVENTS>(&mut world, &leaves);
        });
    });

    group.bench_function("single_event_type_no_listeners", |bencher| {
        let mut world = World::new();
        let (roots, leaves, nodes) = spawn_listener_hierarchy(&mut world);
        add_listeners_to_hierarchy::<DENSITY, 1>(&roots, &leaves, &nodes, &mut world);

        bencher.iter(|| {
            // no listeners to observe TestEvent<9>
            send_events::<9, N_EVENTS>(&mut world, &leaves);
        });
    });

    group.bench_function("four_event_types", |bencher| {
        let mut world = World::new();
        let (roots, leaves, nodes) = spawn_listener_hierarchy(&mut world);
        const FRAC_N_EVENTS_4: usize = N_EVENTS / 4;
        const FRAC_DENSITY_4: usize = DENSITY / 4;
        add_listeners_to_hierarchy::<FRAC_DENSITY_4, 1>(&roots, &leaves, &nodes, &mut world);
        add_listeners_to_hierarchy::<FRAC_DENSITY_4, 2>(&roots, &leaves, &nodes, &mut world);
        add_listeners_to_hierarchy::<FRAC_DENSITY_4, 3>(&roots, &leaves, &nodes, &mut world);
        add_listeners_to_hierarchy::<FRAC_DENSITY_4, 4>(&roots, &leaves, &nodes, &mut world);

        bencher.iter(|| {
            send_events::<1, FRAC_N_EVENTS_4>(&mut world, &leaves);
            send_events::<2, FRAC_N_EVENTS_4>(&mut world, &leaves);
            send_events::<3, FRAC_N_EVENTS_4>(&mut world, &leaves);
            send_events::<4, FRAC_N_EVENTS_4>(&mut world, &leaves);
        });
    });

    group.finish();
}

#[derive(Clone, Event)]
struct EventBase;
#[derive(Clone, Component)]
struct TestEvent<const N: usize> {}

impl<const N: usize> Event for TestEvent<N> {
    type Traversal = Parent;
    const AUTO_PROPAGATE: bool = true;
}

fn send_events<const N: usize, const N_EVENTS: usize>(world: &mut World, leaves: &Vec<Entity>) {
    let target = leaves.iter().choose(&mut rand::thread_rng()).unwrap();

    (0..N_EVENTS).for_each(|_| {
        world.trigger_targets(TestEvent::<N> {}, *target);
    });
}

fn send_base_event(world: &mut World, entities: &Vec<Entity>) {
    world.trigger_targets(EventBase, entities);
}

fn spawn_listener_hierarchy(world: &mut World) -> (Vec<Entity>, Vec<Entity>, Vec<Entity>) {
    let mut roots = vec![];
    let mut leaves = vec![];
    let mut nodes = vec![];
    for _ in 0..ENTITY_WIDTH {
        let mut parent = world.spawn_empty().id();
        roots.push(parent);
        for _ in 0..ENTITY_DEPTH {
            let child = world.spawn_empty().id();
            nodes.push(child);

            world.entity_mut(parent).add_child(child);
            parent = child;
        }
        nodes.pop();
        leaves.push(parent);
    }
    (roots, leaves, nodes)
}

fn add_listeners_to_hierarchy<const DENSITY: usize, const N: usize>(
    roots: &Vec<Entity>,
    leaves: &Vec<Entity>,
    nodes: &Vec<Entity>,
    world: &mut World,
) {
    for e in roots.iter() {
        world.entity_mut(*e).observe(empty_listener::<N>);
    }
    for e in leaves.iter() {
        world.entity_mut(*e).observe(empty_listener::<N>);
    }
    let mut rng = deterministic_rand();
    for e in nodes.iter() {
        if rng().gen_bool(DENSITY as f64 / 100.0) {
            world.entity_mut(*e).observe(empty_listener::<N>);
        }
    }
}

fn empty_listener<const N: usize>(trigger: Trigger<TestEvent<N>>) {
    black_box(trigger);
}

fn empty_listener_base(trigger: Trigger<EventBase>) {
    black_box(trigger);
}
