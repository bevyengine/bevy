use criterion::criterion_group;

mod run_criteria;
mod running_systems;
mod schedule;

use run_criteria::*;
use running_systems::*;
use schedule::*;

criterion_group!(
    scheduling_benches,
    run_criteria_yes,
    run_criteria_no,
    run_criteria_yes_with_query,
    run_criteria_yes_with_resource,
    empty_systems,
    busy_systems,
    contrived,
    schedule,
    build_schedule,
);
