use criterion::criterion_group;

mod multievent;
mod propagation;
mod simple;
mod untyped;
use multievent::*;
use propagation::*;
use simple::*;
use untyped::*;

criterion_group!(
    observer_benches,
    event_propagation,
    observe_simple,
    observe_multievent,
    observe_untyped
);
