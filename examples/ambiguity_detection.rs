//! A test to confirm that `bevy` doesn't have system order ambiguity with DefaultPlugins
//! This is run in CI to ensure that this doesn't regress again.

use bevy::{
    ecs::schedule::{LogLevel, ScheduleBuildSettings},
    prelude::*,
};

/// A test to confirm that `bevy` doesn't have system order ambiguity with DefaultPlugins
/// This is run in CI to ensure that this doesn't regress again.
pub fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.edit_schedule(PreStartup, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.edit_schedule(Startup, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.edit_schedule(PostStartup, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });

    app.edit_schedule(SpawnScene, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.edit_schedule(First, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.edit_schedule(PreUpdate, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.edit_schedule(Update, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });

    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.edit_schedule(Last, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    });
    app.finish();
    app.cleanup();
    app.update();
    assert!(
        app.get_schedule(PreStartup)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(Startup)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(PostStartup)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(SpawnScene)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(First)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(PreUpdate)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(Update)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(PostUpdate)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
    assert!(
        app.get_schedule(Last)
            .unwrap()
            .graph()
            .conflicting_systems()
            .len()
            == 0
    );
}
