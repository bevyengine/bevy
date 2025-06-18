mod propagation;
mod simple;

use criterion::criterion_group;
use propagation::*;
use simple::*;

criterion_group!(benches, event_propagation, observe_simple);
