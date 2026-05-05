mod combinator_system;
mod dyn_param;
mod param_set;

use combinator_system::*;
use criterion::criterion_group;
use dyn_param::*;
use param_set::*;

criterion_group!(benches, combinator_system, dyn_param, param_set);
