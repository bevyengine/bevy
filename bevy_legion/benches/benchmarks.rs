use criterion::*;
use itertools::*;

use legion::prelude::*;

#[derive(Copy, Clone, Debug, PartialEq)]
struct A(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct B(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct C(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct D(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct E(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct F(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Tag(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Position(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Rotation(f32);

fn create_entities(
    world: &mut World,
    variants: &mut [Box<dyn FnMut(Entity, &mut World)>],
    num_components: usize,
    count: usize,
) {
    let len_variants = variants.len();
    let components = (0..)
        .flat_map(|step| (0..len_variants).map(move |i| (i + i * step) % len_variants))
        .chunks(num_components);

    for initializers in (&components).into_iter().take(count) {
        let entity = world.insert((), Some((A(0.0),)))[0];
        for i in initializers {
            let init = variants.get_mut(i).unwrap();
            init(entity, world);
        }
    }
}

fn add_background_entities(world: &mut World, count: usize) {
    create_entities(
        world,
        &mut [
            Box::new(|e, w| w.add_component(e, A(0.0))),
            Box::new(|e, w| w.add_component(e, B(0.0))),
            Box::new(|e, w| w.add_component(e, C(0.0))),
            Box::new(|e, w| w.add_tag(e, Tag(0.0))),
            Box::new(|e, w| w.add_component(e, D(0.0))),
            Box::new(|e, w| w.add_tag(e, Tag(1.0))),
            Box::new(|e, w| w.add_component(e, E(0.0))),
            Box::new(|e, w| w.add_tag(e, Tag(2.0))),
            Box::new(|e, w| w.add_component(e, F(0.0))),
            Box::new(|e, w| w.add_tag(e, Tag(3.0))),
        ],
        5,
        count,
    );
}

fn setup(n: usize) -> World {
    let universe = Universe::new();
    let mut world = universe.create_world();

    world.insert((), (0..n).map(|_| (Position(0.), Rotation(0.))));

    world
}

fn bench_create_delete(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "create-delete",
        |b, count| {
            let mut world = setup(0);
            b.iter(|| {
                let entities = world
                    .insert((), (0..*count).map(|_| (Position(0.),)))
                    .to_vec();

                for e in entities {
                    world.delete(e);
                }
            })
        },
        (0..10).map(|i| i * 100),
    );
}

fn bench_iter_simple(c: &mut Criterion) {
    c.bench_function("iter-simple", |b| {
        let mut world = setup(2000);
        add_background_entities(&mut world, 10000);

        let query = <(Read<Position>, Write<Rotation>)>::query();

        b.iter(|| {
            for (pos, mut rot) in query.iter_mut(&mut world) {
                rot.0 = pos.0;
            }
        });
    });
}

fn bench_iter_complex(c: &mut Criterion) {
    c.bench_function("iter-complex", |b| {
        let mut world = setup(0);
        add_background_entities(&mut world, 10000);

        for i in 0..200 {
            world.insert(
                (Tag(i as f32),),
                (0..2000).map(|_| (Position(0.), Rotation(0.))),
            );
        }

        let query = <(Read<Position>, Write<Rotation>)>::query()
            .filter(!component::<A>() & tag_value(&Tag(2.0)));

        b.iter(|| {
            for (pos, mut rot) in query.iter_mut(&mut world) {
                rot.0 = pos.0;
            }
        });
    });
}

fn bench_iter_chunks_simple(c: &mut Criterion) {
    c.bench_function("iter-chunks-simple", |b| {
        let mut world = setup(10000);
        add_background_entities(&mut world, 10000);

        let query = <(Write<Position>, Read<Rotation>)>::query();

        b.iter(|| {
            for c in query.iter_chunks_mut(&mut world) {
                unsafe {
                    c.components_mut::<Position>()
                        .unwrap()
                        .get_unchecked_mut(0)
                        .0 = 0.0
                };
            }
        });
    });
}

fn bench_iter_chunks_complex(c: &mut Criterion) {
    c.bench_function("iter-chunks-complex", |b| {
        let mut world = setup(0);
        add_background_entities(&mut world, 10000);

        for i in 0..200 {
            world.insert(
                (Tag(i as f32),),
                (0..10000).map(|_| (Position(0.), Rotation(0.))),
            );
        }

        let query = <(Write<Position>, Read<Rotation>)>::query()
            .filter(!component::<A>() & tag_value(&Tag(2.0)));

        b.iter(|| {
            for c in query.iter_chunks_mut(&mut world) {
                unsafe {
                    c.components_mut::<Position>()
                        .unwrap()
                        .get_unchecked_mut(0)
                        .0 = 0.0
                };
            }
        });
    });
}

criterion_group!(
    basic,
    bench_create_delete,
    bench_iter_simple,
    bench_iter_complex,
    bench_iter_chunks_simple,
    bench_iter_chunks_complex
);
criterion_main!(basic);
