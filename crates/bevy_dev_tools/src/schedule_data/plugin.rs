//! Convenience plugin for automatically performing serialization of schedules on boot.
use std::{fs::File, io::Write, path::PathBuf};

use bevy_app::{App, Main, Plugin};
use bevy_ecs::{
    error::BevyError,
    intern::Interned,
    resource::Resource,
    schedule::{
        common_conditions::run_once, IntoScheduleConfigs, ScheduleLabel, Schedules, SystemSet,
    },
    world::World,
};
use ron::ser::PrettyConfig;

use crate::schedule_data::serde::AppData;

/// A plugin to automatically collect and write all schedule data on boot to a file that can later
/// be parsed.
pub struct SerializeSchedulesPlugin {
    /// The schedule that systems will be added to.
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
                Main,
                collect_system_data
                    .run_if(run_once)
                    .in_set(SerializeSchedulesSystems)
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
    for label in labels {
        let mut schedules = world.resource_mut::<Schedules>();
        let mut schedule = schedules.remove(label).unwrap();
        schedule.initialize(world)?;

        let mut schedules = world.resource_mut::<Schedules>();
        schedules.insert(schedule);
    }

    let schedules = world.resource::<Schedules>();
    let app_data = AppData::from_schedules(schedules, world.components())?;

    let file_path = world
        .get_resource::<SerializeSchedulesFilePath>()
        .ok_or("Missing SerializeSchedulesFilePath resource")?;
    let mut file = File::create(&file_path.0)?;
    // Use \n unconditionally so that Windows formatting is predictable.
    let serialized = ron::ser::to_string_pretty(&app_data, PrettyConfig::default().new_line("\n"))?;
    file.write_all(serialized.as_bytes())?;

    Ok(())
}
