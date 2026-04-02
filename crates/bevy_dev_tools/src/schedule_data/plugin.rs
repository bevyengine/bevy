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
/// By default, the schedule data is written to "<current working directory>/app_data.ron". This can
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

fn collect_system_data(world: &mut World) -> Result<(), BevyError> {
    let schedules = world.resource::<Schedules>();
    let labels = schedules
        .iter()
        .map(|schedule| schedule.1.label())
        .collect::<Vec<_>>();
    let mut label_to_build_metadata = HashMap::new();

    for label in labels {
        let mut schedules = world.resource_mut::<Schedules>();
        let mut schedule = schedules.remove(label).unwrap();
        let Some(build_metadata) = schedule.initialize(world)? else {
            return Err(BevyError::from(
                "The schedule has already been built, so we can't collect its system data",
            )
            .with_severity(Severity::Warning));
        };

        label_to_build_metadata.insert(label, build_metadata);

        let mut schedules = world.resource_mut::<Schedules>();
        schedules.insert(schedule);
    }

    let schedules = world.resource::<Schedules>();
    let app_data = AppData::from_schedules(schedules, world.components(), &label_to_build_metadata)
        .with_severity(Severity::Warning)?;

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
