use bevy_reflect::func::{ArgList, IntoFunction, TypedFunction};
use bevy_reflect::prelude::*;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

criterion_group!(benches, typed, into, call, clone);
criterion_main!(benches);

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn typed(c: &mut Criterion) {
    c.benchmark_group("typed")
        .bench_function("function", |b| {
            b.iter(|| add.get_function_info());
        })
        .bench_function("closure", |b| {
            let capture = 25;
            let closure = |a: i32| a + capture;
            b.iter(|| closure.get_function_info());
        });
}

fn into(c: &mut Criterion) {
    c.benchmark_group("into")
        .bench_function("function", |b| {
            b.iter(|| add.into_function());
        })
        .bench_function("closure", |b| {
            let capture = 25;
            let closure = |a: i32| a + capture;
            b.iter(|| closure.into_function());
        });
}

fn call(c: &mut Criterion) {
    c.benchmark_group("call")
        .bench_function("function", |b| {
            let add = add.into_function();
            b.iter_batched(
                || ArgList::new().push_owned(75_i32).push_owned(25_i32),
                |args| add.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function("closure", |b| {
            let capture = 25;
            let add = (|a: i32| a + capture).into_function();
            b.iter_batched(
                || ArgList::new().push_owned(75_i32),
                |args| add.call(args),
                BatchSize::SmallInput,
            );
        });
}

fn clone(c: &mut Criterion) {
    c.benchmark_group("clone").bench_function("function", |b| {
        let add = add.into_function();
        b.iter(|| add.clone());
    });
}
