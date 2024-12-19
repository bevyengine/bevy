#![expect(clippy::type_complexity)]

use criterion::criterion_main;

#[cfg(feature = "reflect_functions")]
mod function;
mod list;
mod map;
mod path;
mod r#struct;

criterion_main!(
    function_benches,
    list::benches,
    map::benches,
    path::benches,
    r#struct::benches,
);

// The `criterion_main!` macro doesn't support `#[cfg(...)]` annotations, so we have to create a
// separate function instead.
fn function_benches() {
    #[cfg(feature = "reflect_functions")]
    function::benches();
}
