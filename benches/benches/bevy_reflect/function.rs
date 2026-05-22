use core::hint::black_box;

use benches::bench;
use bevy_reflect::func::{ArgList, IntoFunction, IntoFunctionMut, TypedFunction};
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion};

criterion_group!(
    benches,
    typed,
    into,
    call,
    clone,
    with_overload,
    call_overload,
);

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn typed(c: &mut Criterion) {
    c.benchmark_group(bench!("typed"))
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
    c.benchmark_group(bench!("into"))
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
            // `move` is required here because `into_function_mut()` takes ownership of `self`.
            let closure = move |a: i32| _capture += a;
            b.iter(|| closure.into_function_mut());
        });
}

fn call(c: &mut Criterion) {
    c.benchmark_group(bench!("call"))
        .bench_function("trait_object", |b| {
            b.iter_batched(
                || Box::new(add) as Box<dyn Fn(i32, i32) -> i32>,
                |func| func(black_box(75), black_box(25)),
                BatchSize::SmallInput,
            );
        })
        .bench_function("function", |b| {
            let add = add.into_function();
            b.iter_batched(
                || ArgList::new().with_owned(75_i32).with_owned(25_i32),
                |args| add.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function("closure", |b| {
            let capture = 25;
            let add = (|a: i32| a + capture).into_function();
            b.iter_batched(
                || ArgList::new().with_owned(75_i32),
                |args| add.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function("closure_mut", |b| {
            let mut capture = 25;
            let mut add = (|a: i32| capture += a).into_function_mut();
            b.iter_batched(
                || ArgList::new().with_owned(75_i32),
                |args| add.call(args),
                BatchSize::SmallInput,
            );
        });
}

fn clone(c: &mut Criterion) {
    c.benchmark_group(bench!("clone"))
        .bench_function("function", |b| {
            let add = add.into_function();
            b.iter(|| add.clone());
        });
}

fn simple<T: std::ops::Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

fn complex<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9>(
    _: T0,
    _: T1,
    _: T2,
    _: T3,
    _: T4,
    _: T5,
    _: T6,
    _: T7,
    _: T8,
    _: T9,
) {
}

fn with_overload(c: &mut Criterion) {
    c.benchmark_group(bench!("with_overload"))
        .bench_function(BenchmarkId::new("simple_overload", 1), |b| {
            b.iter_batched(
                || simple::<i8>.into_function(),
                |func| func.with_overload(simple::<i16>),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("complex_overload", 1), |b| {
            b.iter_batched(
                || complex::<i8, i16, i32, i64, i128, u8, u16, u32, u64, u128>.into_function(),
                |func| {
                    func.with_overload(complex::<i16, i32, i64, i128, u8, u16, u32, u64, u128, i8>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("simple_overload", 3), |b| {
            b.iter_batched(
                || simple::<i8>.into_function(),
                |func| {
                    func.with_overload(simple::<i16>)
                        .with_overload(simple::<i32>)
                        .with_overload(simple::<i64>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("complex_overload", 3), |b| {
            b.iter_batched(
                || complex::<i8, i16, i32, i64, i128, u8, u16, u32, u64, u128>.into_function(),
                |func| {
                    func.with_overload(complex::<i16, i32, i64, i128, u8, u16, u32, u64, u128, i8>)
                        .with_overload(complex::<i32, i64, i128, u8, u16, u32, u64, u128, i8, i16>)
                        .with_overload(complex::<i64, i128, u8, u16, u32, u64, u128, i8, i16, i32>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("simple_overload", 10), |b| {
            b.iter_batched(
                || simple::<i8>.into_function(),
                |func| {
                    func.with_overload(simple::<i16>)
                        .with_overload(simple::<i32>)
                        .with_overload(simple::<i64>)
                        .with_overload(simple::<i128>)
                        .with_overload(simple::<u8>)
                        .with_overload(simple::<u16>)
                        .with_overload(simple::<u32>)
                        .with_overload(simple::<u64>)
                        .with_overload(simple::<u128>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("complex_overload", 10), |b| {
            b.iter_batched(
                || complex::<i8, i16, i32, i64, i128, u8, u16, u32, u64, u128>.into_function(),
                |func| {
                    func.with_overload(complex::<i16, i32, i64, i128, u8, u16, u32, u64, u128, i8>)
                        .with_overload(complex::<i32, i64, i128, u8, u16, u32, u64, u128, i8, i16>)
                        .with_overload(complex::<i64, i128, u8, u16, u32, u64, u128, i8, i16, i32>)
                        .with_overload(complex::<i128, u8, u16, u32, u64, u128, i8, i16, i32, i64>)
                        .with_overload(complex::<u8, u16, u32, u64, u128, i8, i16, i32, i64, i128>)
                        .with_overload(complex::<u16, u32, u64, u128, i8, i16, i32, i64, i128, u8>)
                        .with_overload(complex::<u32, u64, u128, i8, i16, i32, i64, i128, u8, u16>)
                        .with_overload(complex::<u64, u128, i8, i16, i32, i64, i128, u8, u16, u32>)
                        .with_overload(complex::<u128, i8, i16, i32, i64, i128, u8, u16, u32, u64>)
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("nested_simple_overload", 1), |b| {
            b.iter_batched(
                || simple::<i8>.into_function(),
                |func| func.with_overload(simple::<i16>),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("nested_simple_overload", 3), |b| {
            b.iter_batched(
                || simple::<i8>.into_function(),
                |func| {
                    func.with_overload(
                        simple::<i16>.into_function().with_overload(
                            simple::<i32>.into_function().with_overload(simple::<i64>),
                        ),
                    )
                },
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("nested_simple_overload", 10), |b| {
            b.iter_batched(
                || simple::<i8>.into_function(),
                |func| {
                    func.with_overload(
                        simple::<i16>.into_function().with_overload(
                            simple::<i32>.into_function().with_overload(
                                simple::<i64>.into_function().with_overload(
                                    simple::<i128>.into_function().with_overload(
                                        simple::<u8>.into_function().with_overload(
                                            simple::<u16>.into_function().with_overload(
                                                simple::<u32>.into_function().with_overload(
                                                    simple::<u64>
                                                        .into_function()
                                                        .with_overload(simple::<u128>),
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
}

fn call_overload(c: &mut Criterion) {
    c.benchmark_group(bench!("call_overload"))
        .bench_function(BenchmarkId::new("simple_overload", 1), |b| {
            b.iter_batched(
                || {
                    (
                        simple::<i8>.into_function().with_overload(simple::<i16>),
                        ArgList::new().with_owned(75_i8).with_owned(25_i8),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("complex_overload", 1), |b| {
            b.iter_batched(
                || {
                    (
                        complex::<i8, i16, i32, i64, i128, u8, u16, u32, u64, u128>
                            .into_function()
                            .with_overload(
                                complex::<i16, i32, i64, i128, u8, u16, u32, u64, u128, i8>,
                            ),
                        ArgList::new()
                            .with_owned(1_i8)
                            .with_owned(2_i16)
                            .with_owned(3_i32)
                            .with_owned(4_i64)
                            .with_owned(5_i128)
                            .with_owned(6_u8)
                            .with_owned(7_u16)
                            .with_owned(8_u32)
                            .with_owned(9_u64)
                            .with_owned(10_u128),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("simple_overload", 3), |b| {
            b.iter_batched(
                || {
                    (
                        simple::<i8>
                            .into_function()
                            .with_overload(simple::<i16>)
                            .with_overload(simple::<i32>)
                            .with_overload(simple::<i64>),
                        ArgList::new().with_owned(75_i32).with_owned(25_i32),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("complex_overload", 3), |b| {
            b.iter_batched(
                || {
                    (
                        complex::<i8, i16, i32, i64, i128, u8, u16, u32, u64, u128>
                            .into_function()
                            .with_overload(
                                complex::<i16, i32, i64, i128, u8, u16, u32, u64, u128, i8>,
                            )
                            .with_overload(
                                complex::<i32, i64, i128, u8, u16, u32, u64, u128, i8, i16>,
                            )
                            .with_overload(
                                complex::<i64, i128, u8, u16, u32, u64, u128, i8, i16, i32>,
                            ),
                        ArgList::new()
                            .with_owned(1_i32)
                            .with_owned(2_i64)
                            .with_owned(3_i128)
                            .with_owned(4_u8)
                            .with_owned(5_u16)
                            .with_owned(6_u32)
                            .with_owned(7_u64)
                            .with_owned(8_u128)
                            .with_owned(9_i8)
                            .with_owned(10_i16),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("simple_overload", 10), |b| {
            b.iter_batched(
                || {
                    (
                        simple::<i8>
                            .into_function()
                            .with_overload(simple::<i16>)
                            .with_overload(simple::<i32>)
                            .with_overload(simple::<i64>)
                            .with_overload(simple::<i128>)
                            .with_overload(simple::<u8>)
                            .with_overload(simple::<u16>)
                            .with_overload(simple::<u32>)
                            .with_overload(simple::<u64>)
                            .with_overload(simple::<u128>),
                        ArgList::new().with_owned(75_u8).with_owned(25_u8),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        })
        .bench_function(BenchmarkId::new("complex_overload", 10), |b| {
            b.iter_batched(
                || {
                    (
                        complex::<i8, i16, i32, i64, i128, u8, u16, u32, u64, u128>
                            .into_function()
                            .with_overload(
                                complex::<i16, i32, i64, i128, u8, u16, u32, u64, u128, i8>,
                            )
                            .with_overload(
                                complex::<i32, i64, i128, u8, u16, u32, u64, u128, i8, i16>,
                            )
                            .with_overload(
                                complex::<i64, i128, u8, u16, u32, u64, u128, i8, i16, i32>,
                            )
                            .with_overload(
                                complex::<i128, u8, u16, u32, u64, u128, i8, i16, i32, i64>,
                            )
                            .with_overload(
                                complex::<u8, u16, u32, u64, u128, i8, i16, i32, i64, i128>,
                            )
                            .with_overload(
                                complex::<u16, u32, u64, u128, i8, i16, i32, i64, i128, u8>,
                            )
                            .with_overload(
                                complex::<u32, u64, u128, i8, i16, i32, i64, i128, u8, u16>,
                            )
                            .with_overload(
                                complex::<u64, u128, i8, i16, i32, i64, i128, u8, u16, u32>,
                            )
                            .with_overload(
                                complex::<u128, i8, i16, i32, i64, i128, u8, u16, u32, u64>,
                            ),
                        ArgList::new()
                            .with_owned(1_u8)
                            .with_owned(2_u16)
                            .with_owned(3_u32)
                            .with_owned(4_u64)
                            .with_owned(5_u128)
                            .with_owned(6_i8)
                            .with_owned(7_i16)
                            .with_owned(8_i32)
                            .with_owned(9_i64)
                            .with_owned(10_i128),
                    )
                },
                |(func, args)| func.call(args),
                BatchSize::SmallInput,
            );
        });
}
