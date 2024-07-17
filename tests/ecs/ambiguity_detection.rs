//! A test to confirm that `bevy` doesn't regress its system ambiguities count when using [`DefaultPlugins`].
//! This is run in CI.

use bevy::{
    ecs::schedule::{InternedScheduleLabel, LogLevel, ScheduleBuildSettings, ScheduleLabel},
    prelude::*,
    utils::HashMap,
};
use bevy_render::{pipelined_rendering::RenderExtractApp, Render, RenderApp};

/// FIXME: bevy should not have any ambiguities, but it takes time to clean these up,
/// so we're juste ignoring those for now.
///
/// See [#7386](https://github.com/bevyengine/bevy/issues/7386) for relevant issue.
pub fn get_ignored_ambiguous_system_schedules() -> Vec<Box<dyn ScheduleLabel>> {
    vec![
        Box::new(First),
        Box::new(PreUpdate),
        Box::new(Update),
        Box::new(PostUpdate),
        Box::new(Last),
        Box::new(ExtractSchedule),
        Box::new(Render),
    ]
}

/// A test to confirm that `bevy` doesn't regress its system ambiguities count when using [`DefaultPlugins`].
/// This is run in CI.
pub fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let sub_app = app.main_mut();
    configure_ambiguity_detection(sub_app);
    let sub_app = app.sub_app_mut(RenderApp);
    configure_ambiguity_detection(sub_app);
    let sub_app = app.sub_app_mut(RenderExtractApp);
    configure_ambiguity_detection(sub_app);

    app.finish();
    app.cleanup();
    app.update();

    let sub_app = app.main();

    let ignored_schedules = get_ignored_ambiguous_system_schedules();

    let ambiguities = count_ambiguities(sub_app);
    let mut unexpected_ambiguities = vec![];
    for kv in ambiguities.0.iter() {
        if ignored_schedules.iter().any(|label| **label == **kv.0) {
            continue;
        }
        if *kv.1 != 0 {
            unexpected_ambiguities.push(kv.0);
        }
    }
    assert_eq!(
        unexpected_ambiguities.len(),
        0,
        "Main app has unexpected ambiguities among these schedules: {:?}.\n\
    More Details:\n{:#?}",
        unexpected_ambiguities,
        ambiguities
    );

    let total_ambiguities: usize = ambiguities.0.values().sum();
    assert_eq!(
        total_ambiguities, 82,
        "Main app does not have an expected conflicting systems count, \
        you might consider verifying if it's normal, or change the expected number.\n\
        Details:\n{:#?}",
        ambiguities
    );

    // RenderApp is not checked here, because it is not within the App at this point.
    let sub_app = app.sub_app(RenderExtractApp);

    let ambiguities = count_ambiguities(sub_app);
    let total_ambiguities: usize = ambiguities.0.values().sum();
    assert_eq!(
        total_ambiguities, 0,
        "RenderExtractApp contains conflicting systems.",
    );
}

/// Contains the number of conflicting systems per schedule.
#[derive(Debug)]
struct AmibiguitiesCount(pub HashMap<InternedScheduleLabel, usize>);

fn configure_ambiguity_detection(sub_app: &mut SubApp) {
    let ignored_ambiguous_systems = get_ignored_ambiguous_system_schedules();
    let mut schedules = sub_app.world_mut().resource_mut::<Schedules>();
    for (_, schedule) in schedules.iter_mut() {
        if ignored_ambiguous_systems
            .iter()
            .any(|label| **label == *schedule.label())
        {
            // Note: you can remove this bypass to get full details about ambiguities.
            continue;
        }
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    }
}

/// Returns the number of conflicting systems per schedule.
fn count_ambiguities(sub_app: &SubApp) -> AmibiguitiesCount {
    let schedules = sub_app.world().resource::<Schedules>();
    let mut ambiguities: bevy::utils::hashbrown::HashMap<InternedScheduleLabel, usize> =
        HashMap::new();
    for (_, schedule) in schedules.iter() {
        let ambiguities_in_schedule = schedule.graph().conflicting_systems().len();
        ambiguities.insert(schedule.label(), ambiguities_in_schedule);
    }
    AmibiguitiesCount(ambiguities)
}
