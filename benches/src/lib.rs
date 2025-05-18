/// Automatically generates the qualified name of a benchmark given its function name and module
/// path.
///
/// This macro takes a single string literal as input and returns a [`&'static str`](str). Its
/// result is determined at compile-time. If you need to create variations of a benchmark name
/// based on its input, use this in combination with [`BenchmarkId`](criterion::BenchmarkId).
///
/// # When to use this
///
/// Use this macro to name benchmarks that are not within a group and benchmark groups themselves.
/// You'll most commonly use this macro with:
///
/// - [`Criterion::bench_function()`](criterion::Criterion::bench_function)
/// - [`Criterion::bench_with_input()`](criterion::Criterion::bench_with_input)
/// - [`Criterion::benchmark_group()`](criterion::Criterion::benchmark_group)
///
/// You do not want to use this macro with
/// [`BenchmarkGroup::bench_function()`](criterion::BenchmarkGroup::bench_function) or
/// [`BenchmarkGroup::bench_with_input()`](criterion::BenchmarkGroup::bench_with_input), because
/// the group they are in already has the qualified path in it.
///
/// # Example
///
/// ```
/// mod ecs {
///     mod query {
///         use criterion::Criterion;
///         use benches::bench;
///
///         fn iter(c: &mut Criterion) {
///             // Benchmark name ends in `ecs::query::iter`.
///             c.bench_function(bench!("iter"), |b| {
///                 // ...
///             });
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! bench {
    ($name:literal) => {
        concat!(module_path!(), "::", $name)
    };
}
