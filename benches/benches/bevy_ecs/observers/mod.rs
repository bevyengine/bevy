use criterion::criterion_group;

mod dynamic;
mod multievent;
mod propagation;
mod semidynamic;
mod simple;
use dynamic::*;
use multievent::*;
use propagation::*;
use semidynamic::*;
use simple::*;

criterion_group!(
    observer_benches,
    event_propagation,
    observe_simple,
    observe_multievent,
    observe_dynamic,
    observe_semidynamic
);
