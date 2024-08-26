use criterion::criterion_group;

mod dynamic;
mod propagation;
mod simple;
use dynamic::*;
use propagation::*;
use simple::*;

criterion_group!(
    observer_benches,
    event_propagation,
    observe_simple,
    observe_dynamic,
);
