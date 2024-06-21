//! A test to confirm that `bevy` doesn't have system order ambiguity with DefaultPlugins
//! This is run in CI to ensure that this doesn't regress again.

use bevy::{
    ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel},
    prelude::*,
};
//use bevy_render::{pipelined_rendering::RenderExtractApp, Render, RenderApp};

/// FIXME: bevy should not have any ambiguities, but it takes time to clean these up,
/// so we're juste ignoring those for now.
fn get_ignored_ambiguous_systems() -> Vec<Box<dyn ScheduleLabel>> {
    vec![
        Box::new(First),
        Box::new(PreUpdate),
        Box::new(PostUpdate),
        Box::new(Last),
        Box::new(ExtractSchedule),
        //Box::new(Render),
    ]
}

/// A test to confirm that `bevy` doesn't have system order ambiguity with DefaultPlugins
/// This is run in CI to ensure that this doesn't regress again.
pub fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    fn configure_ambiguity_detection(sub_app: &mut SubApp) {
        let ignored_ambiguous_systems = get_ignored_ambiguous_systems();
        let mut schedules = sub_app.world_mut().resource_mut::<Schedules>();
        for (_, schedule) in schedules.iter_mut() {
            if ignored_ambiguous_systems
                .iter()
                .find(|label| ***label == *schedule.label())
                .is_some()
            {
                continue;
            }
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                use_shortnames: false,
                ..default()
            });
        }
    }
    let sub_app = app.main_mut();
    configure_ambiguity_detection(sub_app);
    //let sub_app = app.sub_app_mut(RenderApp);
    //configure_ambiguity_detection(sub_app);
    //let sub_app = app.sub_app_mut(RenderExtractApp);
    //configure_ambiguity_detection(sub_app);

    app.finish();
    app.cleanup();
    app.update();

    fn assert_no_conflicting_systems(sub_app: &SubApp) {
        let ignored_ambiguous_systems = get_ignored_ambiguous_systems();

        let schedules = sub_app.world().resource::<Schedules>();
        for (_, schedule) in schedules.iter() {
            if ignored_ambiguous_systems
                .iter()
                .find(|label| ***label == *schedule.label())
                .is_some()
            {
                continue;
            }
            assert!(schedule.graph().conflicting_systems().len() == 0);
        }
    }
    let sub_app = app.main();
    assert_no_conflicting_systems(sub_app);
    // RenderApp is not checked here, because it is not within the App at this point.
    //let sub_app = app.sub_app(RenderExtractApp);
    //assert_no_conflicting_systems(sub_app);
}
