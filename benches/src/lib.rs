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

/// Automatically generates the qualified name of a benchmark given string formatting arguments and
/// its module path.
///
/// This macro takes string formatting arguments (see [`std::fmt`]) as input and returns a
/// [String]. Its results are determined at runtime, so there is a small cost to using this macro.
///
/// If you just want to generate variants of the same benchmark name for different inputs (which is
/// most common), please use [`BenchmarkId`](criterion::BenchmarkId) instead.
#[macro_export]
#[deprecated = "Prefer to use `criterion`'s built-in `BenchmarkId` instead."]
macro_rules! format_bench {
    ($($arg:tt)*) => {{
        let mut qualified_name = concat!(module_path!(), "::").to_string();
        qualified_name.push_str(format!($($arg)*));
        qualified_name
    }};
}
