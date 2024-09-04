use bevy_reflect::func::{ArgList, IntoFunction, IntoFunctionMut, TypedFunction};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

criterion_group!(benches, typed, into, call, overload, clone);
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
        })
        .bench_function("closure_mut", |b| {
            let mut capture = 25;
            let closure = |a: i32| capture += a;
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
        })
        .bench_function("closure_mut", |b| {
            let mut _capture = 25;
            let closure = move |a: i32| _capture += a;
            b.iter(|| closure.into_function_mut());
        });
}

fn call(c: &mut Criterion) {
    c.benchmark_group("call")
        .bench_function("trait_object", |b| {
            b.iter_batched(
                || Box::new(add) as Box<dyn Fn(i32, i32) -> i32>,
                |func| func(75, 25),
                BatchSize::SmallInput,
            );
        })
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
        })
        .bench_function("closure_mut", |b| {
            let mut capture = 25;
            let mut add = (|a: i32| capture += a).into_function_mut();
            b.iter_batched(
                || ArgList::new().push_owned(75_i32),
                |args| add.call(args),
                BatchSize::SmallInput,
            );
        });
}

fn overload(c: &mut Criterion) {
    fn add<T: std::ops::Add<Output = T>>(a: T, b: T) -> T {
        a + b
    }

    c.benchmark_group("with_overload")
        .bench_function("01_overload", |b| {
            b.iter_batched(
                || add::<i8>.into_function(),
                |func| func.with_overload(add::<i16>),
                BatchSize::SmallInput,
            );
        })
        .bench_function("03_overload", |b| {
            b.iter_batched(
                || add::<i8>.into_function(),
                |func| {
                    func.with_overload(add::<i16>)
                        .with_overload(add::<i32>)
                        .with_overload(add::<i64>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function("10_overload", |b| {
            b.iter_batched(
                || add::<i8>.into_function(),
                |func| {
                    func.with_overload(add::<i16>)
                        .with_overload(add::<i32>)
                        .with_overload(add::<i64>)
                        .with_overload(add::<i128>)
                        .with_overload(add::<u8>)
                        .with_overload(add::<u16>)
                        .with_overload(add::<u32>)
                        .with_overload(add::<u64>)
                        .with_overload(add::<u128>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function("01_nested_overload", |b| {
            b.iter_batched(
                || add::<i8>.into_function(),
                |func| func.with_overload(add::<i16>),
                BatchSize::SmallInput,
            );
        })
        .bench_function("03_nested_overload", |b| {
            b.iter_batched(
                || add::<i8>.into_function(),
                |func| {
                    func.with_overload(
                        add::<i16>
                            .into_function()
                            .with_overload(add::<i32>.into_function().with_overload(add::<i64>)),
                    )
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function("10_nested_overload", |b| {
            b.iter_batched(
                || add::<i8>.into_function(),
                |func| {
                    func.with_overload(
                        add::<i16>.into_function().with_overload(
                            add::<i32>.into_function().with_overload(
                                add::<i64>.into_function().with_overload(
                                    add::<i128>.into_function().with_overload(
                                        add::<u8>.into_function().with_overload(
                                            add::<u16>.into_function().with_overload(
                                                add::<u32>.into_function().with_overload(
                                                    add::<u64>
                                                        .into_function()
                                                        .with_overload(add::<u128>),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    )
                },
                BatchSize::SmallInput,
            );
        });

    c.benchmark_group("call_overload")
        .bench_function("01_overload", |b| {
            b.iter_batched(
                || {
                    (
                        add::<i8>.into_function().with_overload(add::<i16>),
                        ArgList::new().push_owned(75_i8).push_owned(25_i8),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function("03_overload", |b| {
            b.iter_batched(
                || {
                    (
                        add::<i8>
                            .into_function()
                            .with_overload(add::<i16>)
                            .with_overload(add::<i32>)
                            .with_overload(add::<i64>),
                        ArgList::new().push_owned(75_i32).push_owned(25_i32),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function("10_overload", |b| {
            b.iter_batched(
                || {
                    (
                        add::<i8>
                            .into_function()
                            .with_overload(add::<i16>)
                            .with_overload(add::<i32>)
                            .with_overload(add::<i64>)
                            .with_overload(add::<i128>)
                            .with_overload(add::<u8>)
                            .with_overload(add::<u16>)
                            .with_overload(add::<u32>)
                            .with_overload(add::<u64>)
                            .with_overload(add::<u128>),
                        ArgList::new().push_owned(75_u8).push_owned(25_u8),
                    )
                },
                |(func, args)| func.call(args),
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
