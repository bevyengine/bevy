use criterion::criterion_group;

mod propagation;
use propagation::*;

criterion_group!(observer_benches, event_propagation);
