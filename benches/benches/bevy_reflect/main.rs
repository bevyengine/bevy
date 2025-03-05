use criterion::criterion_main;

mod function;
mod list;
mod map;
mod path;
mod r#struct;

criterion_main!(
    function::benches,
    list::benches,
    map::benches,
    path::benches,
    r#struct::benches,
);
