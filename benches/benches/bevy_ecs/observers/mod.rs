mod custom;
mod lifecycle;
mod propagation;

use criterion::criterion_group;
use custom::*;
use lifecycle::*;
use propagation::*;

criterion_group!(
    benches,
    event_propagation,
    observer_custom,
    observer_lifecycle
);
