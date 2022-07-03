use criterion::criterion_group;

mod heavy_compute;
mod schedule;

use heavy_compute::*;
use schedule::*;

criterion_group!(systems_benches, schedule, build_schedule, heavy_compute);
