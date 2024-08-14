//! A test to confirm that `bevy` doesn't regress its system ambiguities count when using [`DefaultPlugins`].
//! This is run in CI.

use bevy::{
    ecs::schedule::{InternedScheduleLabel, LogLevel, ScheduleBuildSettings},
    prelude::*,
    utils::HashMap,
};
use bevy_render::{pipelined_rendering::RenderExtractApp, RenderApp};

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

    let ambiguities = count_ambiguities(sub_app);
    let mut unexpected_ambiguities = vec![];
    for (&label, &count) in ambiguities.0.iter() {
        if count != 0 {
            unexpected_ambiguities.push(label);
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

    let total_ambiguities = ambiguities.total();
    assert_eq!(
        total_ambiguities, 0,
        "No system order ambiguities should be present between systems added in `DefaultPlugins`.\n
        Details:\n{:#?}",
        ambiguities
    );

    // RenderApp is not checked here, because it is not within the App at this point.
    let sub_app = app.sub_app(RenderExtractApp);

    let ambiguities = count_ambiguities(sub_app);
    let total_ambiguities = ambiguities.total();
    assert_eq!(
        total_ambiguities, 0,
        "RenderExtractApp contains conflicting systems.",
    );
}

/// Contains the number of conflicting systems per schedule.
#[derive(Debug, Deref, DerefMut)]
struct AmbiguitiesCount(pub HashMap<InternedScheduleLabel, usize>);

impl AmbiguitiesCount {
    fn total(&self) -> usize {
        self.values().sum()
    }
}

fn configure_ambiguity_detection(sub_app: &mut SubApp) {
    let mut schedules = sub_app.world_mut().resource_mut::<Schedules>();
    for (_, schedule) in schedules.iter_mut() {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    }
}

/// Returns the number of conflicting systems per schedule.
fn count_ambiguities(sub_app: &SubApp) -> AmbiguitiesCount {
    let schedules = sub_app.world().resource::<Schedules>();
    let mut ambiguities = HashMap::new();
    for (_, schedule) in schedules.iter() {
        let ambiguities_in_schedule = schedule.graph().conflicting_systems().len();
        ambiguities.insert(schedule.label(), ambiguities_in_schedule);
    }
    AmbiguitiesCount(ambiguities)
}
