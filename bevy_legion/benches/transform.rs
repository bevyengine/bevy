use criterion::*;

use cgmath::prelude::*;
use cgmath::{vec3, Matrix4, Quaternion, Vector3};
use legion::prelude::*;
use rayon::join;

#[derive(Copy, Clone, Debug, PartialEq)]
struct Position(Vector3<f32>);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Orientation(Quaternion<f32>);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Scale(Vector3<f32>);

#[derive(Copy, Clone, Debug, PartialEq)]
struct Transform(Matrix4<f32>);

fn data(n: usize) -> Vec<(Position, Orientation, Scale, Transform)> {
    let mut v = Vec::<(Position, Orientation, Scale, Transform)>::new();

    for _ in 0..n {
        v.push((
            Position(vec3(0.0, 0.0, 0.0)),
            Orientation(Quaternion::new(1.0, 0.0, 0.0, 0.0)),
            Scale(vec3(0.0, 0.0, 0.0)),
            Transform(Matrix4::identity()),
        ));
    }

    v
}

fn setup(data: Vec<(Position, Orientation, Scale, Transform)>) -> World {
    let universe = Universe::new();
    let mut world = universe.create_world();

    world.insert((), data);

    world
}

fn process(
    position: &Vector3<f32>,
    orientation: &Quaternion<f32>,
    scale: &Vector3<f32>,
) -> Matrix4<f32> {
    let rot: Matrix4<f32> = (*orientation).into();
    Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
        * rot
        * Matrix4::from_translation(*position)
}

fn ideal(data: &mut Vec<(Position, Orientation, Scale, Transform)>) {
    for (pos, orient, scale, trans) in data.iter_mut() {
        trans.0 = process(&pos.0, &orient.0, &scale.0);
    }
}

fn sequential(world: &mut World) {
    for (pos, orient, scale, mut trans) in <(
        Read<Position>,
        Read<Orientation>,
        Read<Scale>,
        Write<Transform>,
    )>::query()
    .iter_mut(world)
    {
        trans.0 = process(&pos.0, &orient.0, &scale.0);
    }
}

fn par_for_each_mut(world: &mut World) {
    <(
        Read<Position>,
        Read<Orientation>,
        Read<Scale>,
        Write<Transform>,
    )>::query()
    .par_for_each_mut(world, |(pos, orient, scale, mut trans)| {
        trans.0 = process(&pos.0, &orient.0, &scale.0);
    });
}

fn bench_transform(c: &mut Criterion) {
    c.bench(
        "update transform (experimental)",
        ParameterizedBenchmark::new(
            "ideal sequential",
            |b, n| {
                let mut data = data(*n);
                b.iter(|| ideal(&mut data));
            },
            (1..11).map(|i| i * 1000),
        )
        .with_function("sequential", |b, n| {
            let data = data(*n);
            let mut world = setup(data);
            b.iter(|| sequential(&mut world));
        })
        .with_function("par_for_each_mut", |b, n| {
            let data = data(*n);
            let mut world = setup(data);
            join(|| {}, || b.iter(|| par_for_each_mut(&mut world)));
        }),
    );
}

criterion_group!(iterate, bench_transform);
criterion_main!(iterate);
