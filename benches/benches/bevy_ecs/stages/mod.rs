use criterion::criterion_group;

mod run_criteria;
mod stages;

pub use run_criteria::*;
pub use stages::*;

criterion_group!(
    stages_benches,
    run_criteria_yes,
    run_criteria_no,
    run_criteria_yes_with_labels,
    run_criteria_no_with_labels,
    run_criteria_yes_with_query,
    run_criteria_yes_with_resource,
    empty_systems,
    busy_systems,
    contrived
);
