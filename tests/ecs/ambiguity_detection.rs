//! A test to confirm that `bevy` doesn't regress its system ambiguities count when using [`DefaultPlugins`].
//! This is run in CI.
//!
//! Note that because this test requires rendering, it isn't actually an integration test!
//! Instead, it's secretly an example: you can run this test manually using `cargo run --example ambiguity_detection`.

use bevy::{
    ecs::schedule::{InternedScheduleLabel, LogLevel, ScheduleBuildSettings},
    prelude::*,
    utils::HashMap,
};
use bevy_render::{pipelined_rendering::RenderExtractApp, RenderApp};

fn main() {
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

    let main_app_ambiguities = count_ambiguities(app.main());
    assert_eq!(
        main_app_ambiguities.total(),
        // This number *should* be zero.
        // Over time, we are working to reduce the number: your PR should not increase it.
        // If you decrease this by fixing an ambiguity, reduce the number to prevent regressions.
        // See https://github.com/bevyengine/bevy/issues/7386 for progress.
        72,
        "Main app has unexpected ambiguities among the following schedules: \n{:#?}.",
        main_app_ambiguities,
    );

    // RenderApp is not checked here, because it is not within the App at this point.
    let render_extract_ambiguities = count_ambiguities(app.sub_app(RenderExtractApp));
    assert_eq!(
        render_extract_ambiguities.total(),
        0,
        "RenderExtract app has unexpected ambiguities among the following schedules: \n{:#?}",
        render_extract_ambiguities,
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
            // NOTE: you can change this to `LogLevel::Ignore` to easily see the current number of ambiguities.
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
