use criterion::criterion_group;

mod run_criteria;
mod schedule;
mod stages;

use run_criteria::*;
use schedule::*;
use stages::*;

criterion_group!(
    scheduling_benches,
    run_criteria_yes,
    run_criteria_no,
    run_criteria_yes_with_labels,
    run_criteria_no_with_labels,
    run_criteria_yes_with_query,
    run_criteria_yes_with_resource,
    empty_systems,
    busy_systems,
    contrived,
    schedule,
    build_schedule,
);
