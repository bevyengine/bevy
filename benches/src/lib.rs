/// Automatically generates the qualified name of a benchmark given its function name and module
/// path.
///
/// This macro takes a single string literal as input and returns a [`&'static str`](str). Its
/// result is determined at compile-time. If you need to dynamically generate the name at runtime,
/// and are positive [`BenchmarkId`](criterion::BenchmarkId) do not suit your needs, you can use
/// [`format_bench!`].
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
