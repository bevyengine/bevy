//! Convenience plugin for automatically performing serialization of schedules on boot.
use std::{fs::File, io::Write, path::PathBuf};

use bevy_app::{App, Main, Plugin};
use bevy_ecs::{
    error::{BevyError, ResultSeverityExt, Severity},
    intern::Interned,
    resource::Resource,
    schedule::{
        common_conditions::run_once, IntoScheduleConfigs, ScheduleLabel, Schedules, SystemSet,
    },
    world::World,
};
use bevy_platform::collections::HashMap;
use ron::ser::PrettyConfig;

use crate::schedule_data::serde::AppData;

/// A plugin to automatically collect and write all schedule data on boot to a file that can later
/// be parsed.
///
/// By default, the schedule data is written to `<current working directory>/app_data.ron`. This can
/// be configured to a different path using [`SerializeSchedulesFilePath`].
pub struct SerializeSchedulesPlugin {
    /// The schedule into which the systems for collecting/writing the schedule data are added.
    ///
    /// This schedule **will not** have its schedule data collected, as well as any "parent"
    /// schedules. In order to run a schedule, Bevy removes it from the world, meaning if this
    /// system is added to schedule [`Update`](bevy_app::Update), that schedule and also [`Main`]
    /// will not be included in the [`AppData`]. The default is the [`Main`] schedule since usually
    /// there is only one system ([`Main::run_main`]), so there's very little data to collect.
    ///
    /// Avoid changing this field. This is intended for power-users who might not use the [`Main`]
    /// schedule at all. It may also be worth considering just calling [`AppData::from_schedules`]
    /// manually to ensure a particular schedule is present.
    ///
    /// Usually, this will be set using [`Self::in_schedule`].
    pub schedule: Interned<dyn ScheduleLabel>,
}

impl Default for SerializeSchedulesPlugin {
    fn default() -> Self {
        Self {
            schedule: Main.intern(),
        }
    }
}

impl SerializeSchedulesPlugin {
    /// Creates an instance of [`Self`] that inserts into the specified schedule.
    pub fn in_schedule(label: impl ScheduleLabel) -> Self {
        Self {
            schedule: label.intern(),
        }
    }
}

impl Plugin for SerializeSchedulesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SerializeSchedulesFilePath>()
            .add_systems(
                self.schedule,
                collect_system_data
                    .run_if(run_once)
                    .in_set(SerializeSchedulesSystems)
                    // While we may not be in the `Main` schedule at all, the default is that, so we
                    // should make this work properly in the default case.
                    .before(Main::run_main),
            );
    }
}

/// A system set for allowing users to configure scheduling properties of systems in
/// [`SerializeSchedulesPlugin`].
#[derive(SystemSet, Hash, PartialEq, Eq, Debug, Clone)]
pub struct SerializeSchedulesSystems;

/// The file path where schedules will be written to after collected by
/// [`SerializeSchedulesPlugin`].
#[derive(Resource)]
pub struct SerializeSchedulesFilePath(pub PathBuf);

impl Default for SerializeSchedulesFilePath {
    fn default() -> Self {
        Self("app_data.ron".into())
    }
}

/// The inner part of [`collect_system_data`] that returns the [`AppData`] so we can write tests
/// without needing to write to disk.
fn collect_system_data_inner(world: &mut World) -> Result<AppData, BevyError> {
    let schedules = world.resource::<Schedules>();
    let labels = schedules
        .iter()
        .map(|schedule| schedule.1.label())
        .collect::<Vec<_>>();
    let mut label_to_build_metadata = HashMap::new();

    for label in labels {
        // Hokey pokey the schedule out of the world so we can initialize it. Note: we can't just
        // remove the whole `Schedule` resource since `Schedule::initialize` accesses `Schedules`
        // internally.
        let result = world.schedule_scope(label, |world, schedule| schedule.initialize(world));
        let Some(build_metadata) = result? else {
            return Err(
                "The schedule has already been built, so we can't collect its system data".into(),
            );
        };

        label_to_build_metadata.insert(label, build_metadata);
    }

    let schedules = world.resource::<Schedules>();
    Ok(AppData::from_schedules(
        schedules,
        world.components(),
        &label_to_build_metadata,
    )?)
}

/// A system that collects all the schedule data and writes it to [`SerializeSchedulesFilePath`].
fn collect_system_data(world: &mut World) -> Result<(), BevyError> {
    let app_data = collect_system_data_inner(world).with_severity(Severity::Warning)?;
    let file_path = world
        .get_resource::<SerializeSchedulesFilePath>()
        .ok_or("Missing SerializeSchedulesFilePath resource")
        .with_severity(Severity::Warning)?;
    let mut file = File::create(&file_path.0).with_severity(Severity::Warning)?;
    // Use \n unconditionally so that Windows formatting is predictable.
    let serialized = ron::ser::to_string_pretty(&app_data, PrettyConfig::default().new_line("\n"))?;
    file.write_all(serialized.as_bytes())
        .with_severity(Severity::Warning)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, PostUpdate, Update};

    use crate::schedule_data::{
        plugin::collect_system_data_inner,
        serde::tests::{remove_module_paths, simple_system, sort_app_data},
    };

    #[test]
    fn collects_all_schedules() {
        // Start with an empty app so only our stuff gets added.
        let mut app = App::empty();

        fn a() {}
        fn b() {}
        fn c() {}
        app.add_systems(Update, (a, b));
        app.add_systems(PostUpdate, c);

        // Normally users would use the plugin, but to avoid writing to disk in a test, we just call
        // the inner part of the system directly.
        let mut app_data = collect_system_data_inner(app.world_mut()).unwrap();
        remove_module_paths(&mut app_data);
        sort_app_data(&mut app_data);

        assert_eq!(app_data.schedules.len(), 2);
        let post_update = &app_data.schedules[0];
        assert_eq!(post_update.name, "PostUpdate");
        assert_eq!(post_update.systems, [simple_system("c")]);
        let update = &app_data.schedules[1];
        assert_eq!(update.name, "Update");
        assert_eq!(update.systems, [simple_system("a"), simple_system("b")]);
    }

    #[test]
    fn uses_safe_schedule_scope() {
        // This tests a niche situation where a schedule has already been built when
        // `collect_system_data_inner` runs. Since this method runs before the `Main` schedule, this
        // can only happen if either: a) the user is using a custom schedule, or b) the user runs a
        // schedule from **inside a plugin** - which is extremely cursed. Either way, better to be
        // safe than sorry!

        // Start with an empty app so only our stuff gets added.
        let mut app = App::empty();

        fn a() {}
        app.add_systems(Update, a);
        app.world_mut().run_schedule(Update);

        // Normally users would use the plugin, but to avoid writing to disk in a test, we just call
        // the inner part of the system directly.

        // We expect an error since the schedule has already been built.
        collect_system_data_inner(app.world_mut()).unwrap_err();

        // If the schedule is missing, this would panic! This could happen if there was an error
        // extracting the schedule data, and we didn't hokey-pokey safely.
        app.world_mut().schedule_scope(Update, |_, _| {});
    }
}
