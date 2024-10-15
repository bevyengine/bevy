use criterion::criterion_group;

mod combinator_system;
mod dyn_param;
mod param_set;

use combinator_system::*;
use dyn_param::*;
use param_set::*;

criterion_group!(param_benches, combinator_system, dyn_param, param_set);
