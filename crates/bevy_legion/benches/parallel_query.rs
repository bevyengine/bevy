use criterion::*;

use itertools::*;
use legion::prelude::*;
use rayon::join;

#[derive(Copy, Clone, Debug, PartialEq)]
struct A(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct B(f32);

#[derive(Copy, Clone, Debug, PartialEq)]
struct C(f32);

#[derive(Copy, Clone, Debug)]
enum Variants {
    AB(A, B),
    AC(A, C),
}

fn index(v: Variants) -> u8 {
    match v {
        Variants::AB(_, _) => 0,
        Variants::AC(_, _) => 1,
    }
}

fn generate(i: u8) -> Variants {
    match i {
        0 => Variants::AB(A(0.0), B(0.0)),
        _ => Variants::AC(A(0.0), C(0.0)),
    }
}

fn data(n: usize) -> Vec<Variants> {
    let mut v = Vec::<Variants>::new();

    for _ in 0..n {
        v.push(generate(0));
    }

    for _ in 0..n {
        v.push(generate(1));
    }

    v
}

fn setup(data: &[Variants]) -> World {
    let universe = Universe::new();
    let mut world = universe.create_world();

    for (i, group) in &data.iter().group_by(|x| index(**x)) {
        match i {
            0 => world.insert(
                (),
                group
                    .map(|x| {
                        if let Variants::AB(a, b) = x {
                            (*a, *b)
                        } else {
                            panic!();
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => world.insert(
                (),
                group
                    .map(|x| {
                        if let Variants::AC(a, c) = x {
                            (*a, *c)
                        } else {
                            panic!();
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
        };
    }

    world
}

fn setup_ideal(data: &[Variants]) -> (Vec<(A, B)>, Vec<(A, C)>) {
    let mut ab = Vec::<(A, B)>::new();
    let mut ac = Vec::<(A, C)>::new();

    for v in data {
        match v {
            Variants::AB(a, b) => ab.push((*a, *b)),
            Variants::AC(a, c) => ac.push((*a, *c)),
        };
    }

    (ab, ac)
}

fn ideal(ab: &mut Vec<(A, B)>, ac: &mut Vec<(A, C)>) {
    for (a, b) in ab.iter_mut() {
        b.0 = a.0;
    }

    for (a, c) in ac.iter_mut() {
        c.0 = a.0;
    }
}

fn sequential(world: &mut World) {
    for (mut b, a) in <(Write<B>, Read<A>)>::query().iter_mut(world) {
        b.0 = a.0;
    }

    for (mut c, a) in <(Write<C>, Read<A>)>::query().iter_mut(world) {
        c.0 = a.0;
    }
}

fn parallel(world: &mut World) {
    join(
        || unsafe {
            for (mut b, a) in <(Write<B>, Read<A>)>::query().iter_unchecked(world) {
                b.0 = a.0;
            }
        },
        || unsafe {
            for (mut c, a) in <(Write<C>, Read<A>)>::query().iter_unchecked(world) {
                c.0 = a.0;
            }
        },
    );
}

fn par_for_each_mut(world: &mut World) {
    join(
        || unsafe {
            <(Write<B>, Read<A>)>::query().par_for_each_unchecked(world, |(mut b, a)| {
                b.0 = a.0;
            });
        },
        || unsafe {
            <(Write<C>, Read<A>)>::query().par_for_each_unchecked(world, |(mut c, a)| {
                c.0 = a.0;
            });
        },
    );
}

fn bench_ordered(c: &mut Criterion) {
    c.bench(
        "concurrent queries",
        ParameterizedBenchmark::new(
            "sequential ideal",
            |b, n| {
                let data = data(*n);
                let (mut ab, mut ac) = setup_ideal(&data);
                b.iter(|| ideal(&mut ab, &mut ac));
            },
            (1..11).map(|i| i * 1000),
        )
        .with_function("sequential", |b, n| {
            let data = data(*n);
            let mut world = setup(&data);
            b.iter(|| sequential(&mut world));
        })
        .with_function("parallel", |b, n| {
            let data = data(*n);
            let mut world = setup(&data);
            join(|| {}, || b.iter(|| parallel(&mut world)));
        })
        .with_function("par_for_each_mut", |b, n| {
            let data = data(*n);
            let mut world = setup(&data);
            join(|| {}, || b.iter(|| par_for_each_mut(&mut world)));
        }),
    );
}

criterion_group!(iterate, bench_ordered);
criterion_main!(iterate);
