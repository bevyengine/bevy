use criterion::criterion_group;

mod propagation;
mod simple;
use propagation::*;
use simple::*;

criterion_group!(benches, event_propagation, observe_simple);
