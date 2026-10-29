//! `stepping.*` verbs for the Bevy Remote Protocol.
//!
//! The method names and response types are always available, so clients can drive stepping on a
//! remote app. The handlers need the `bevy_debug_stepping` feature.

use serde::{Deserialize, Serialize};

#[cfg(feature = "bevy_debug_stepping")]
pub use handlers::*;

/// The method path for a `stepping.status` request.
pub const BRP_STEPPING_STATUS: &str = "stepping.status";

/// The method path for a `stepping.enable` request.
///
/// Responds with `null`. The change applies at the start of the next frame, so call
/// `stepping.status` to read it.
pub const BRP_STEPPING_ENABLE: &str = "stepping.enable";

/// The method path for a `stepping.disable` request.
///
/// Responds with `null`, like [`BRP_STEPPING_ENABLE`].
pub const BRP_STEPPING_DISABLE: &str = "stepping.disable";

/// The method path for a `stepping.step_frame` request.
///
/// Responds with `null`, like [`BRP_STEPPING_ENABLE`].
pub const BRP_STEPPING_STEP_FRAME: &str = "stepping.step_frame";

/// The method path for a `stepping.continue_frame` request.
///
/// Responds with `null`, like [`BRP_STEPPING_ENABLE`].
pub const BRP_STEPPING_CONTINUE_FRAME: &str = "stepping.continue_frame";

/// The position of the stepping cursor within the stepping frame.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BrpSteppingCursor {
    /// The label of the schedule the cursor is currently in.
    pub schedule: String,

    /// The name of the system the cursor is currently at.
    pub system: String,
}

/// The response to a `stepping.status` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct BrpSteppingStatusResponse {
    /// Whether system stepping is currently enabled.
    pub enabled: bool,

    /// The labels of the schedules that have been added to `Stepping`.
    pub schedules: Vec<String>,

    /// The current position of the stepping cursor, if there is one.
    pub cursor: Option<BrpSteppingCursor>,
}

#[cfg(feature = "bevy_debug_stepping")]
mod handlers {
    use bevy_ecs::schedule::{InternedScheduleLabel, NodeId, Schedules, Stepping};
    use bevy_ecs::system::In;
    use bevy_ecs::world::World;
    use serde_json::Value;

    use super::{BrpSteppingCursor, BrpSteppingStatusResponse};
    use crate::{BrpError, BrpResult};

    /// Handles a `stepping.status` request coming from a client.
    pub fn stepping_status(In(_params): In<Option<Value>>, world: &World) -> BrpResult {
        let Some(stepping) = world.get_resource::<Stepping>() else {
            return Err(missing_stepping());
        };
        serde_json::to_value(build_stepping_status(world, stepping)).map_err(BrpError::internal)
    }

    /// Handles a `stepping.enable` request coming from a client.
    pub fn stepping_enable(In(_params): In<Option<Value>>, world: &mut World) -> BrpResult {
        update_stepping(world, Stepping::enable)
    }

    /// Handles a `stepping.disable` request coming from a client.
    pub fn stepping_disable(In(_params): In<Option<Value>>, world: &mut World) -> BrpResult {
        update_stepping(world, Stepping::disable)
    }

    /// Handles a `stepping.step_frame` request coming from a client.
    pub fn stepping_step_frame(In(_params): In<Option<Value>>, world: &mut World) -> BrpResult {
        update_stepping(world, Stepping::step_frame)
    }

    /// Handles a `stepping.continue_frame` request coming from a client.
    pub fn stepping_continue_frame(In(_params): In<Option<Value>>, world: &mut World) -> BrpResult {
        update_stepping(world, Stepping::continue_frame)
    }

    /// Queues `update` on the [`Stepping`] resource, which applies it at the start of the next frame.
    fn update_stepping(world: &mut World, update: fn(&mut Stepping) -> &mut Stepping) -> BrpResult {
        let Some(mut stepping) = world.get_resource_mut::<Stepping>() else {
            return Err(missing_stepping());
        };
        update(&mut stepping);
        Ok(Value::Null)
    }

    fn missing_stepping() -> BrpError {
        BrpError::resource_error("The `Stepping` resource is not present in the world")
    }

    /// Collects the current state of `stepping`.
    fn build_stepping_status(world: &World, stepping: &Stepping) -> BrpSteppingStatusResponse {
        let schedules = stepping
            .schedules()
            .map(|labels| labels.iter().map(|label| format!("{label:?}")).collect())
            .unwrap_or_default();

        let cursor = stepping.cursor().map(|(label, node)| BrpSteppingCursor {
            schedule: format!("{label:?}"),
            system: stepping_system_name(world, label, node),
        });

        BrpSteppingStatusResponse {
            enabled: stepping.is_enabled(),
            schedules,
            cursor,
        }
    }

    /// Resolves the name of `node` within `label`, falling back to the node's debug string.
    fn stepping_system_name(world: &World, label: InternedScheduleLabel, node: NodeId) -> String {
        let NodeId::System(key) = node else {
            return format!("{node:?}");
        };

        world
            .get_resource::<Schedules>()
            .and_then(|schedules| schedules.get(label))
            .and_then(|schedule| schedule.systems().ok())
            .and_then(|mut systems| {
                systems.find_map(|(system_key, system)| {
                    (system_key == key).then(|| system.name().to_string())
                })
            })
            .unwrap_or_else(|| format!("{node:?}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use alloc::vec::Vec;
        use bevy_ecs::resource::Resource;
        use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, ScheduleLabel};
        use bevy_ecs::system::{ResMut, SystemState};

        #[derive(ScheduleLabel, Hash, Clone, PartialEq, Eq, Debug)]
        struct TestSchedule;

        #[derive(Resource, Default)]
        struct Runs(Vec<&'static str>);

        fn first_system(mut runs: ResMut<Runs>) {
            runs.0.push("first");
        }

        fn second_system(mut runs: ResMut<Runs>) {
            runs.0.push("second");
        }

        fn status(world: &World) -> BrpSteppingStatusResponse {
            let value = stepping_status(In(None), world).unwrap();
            serde_json::from_value(value).unwrap()
        }

        fn take_runs(world: &mut World) -> Vec<&'static str> {
            core::mem::take(&mut world.resource_mut::<Runs>().0)
        }

        fn cursor_system(world: &World) -> Option<String> {
            status(world).cursor.map(|cursor| cursor.system)
        }

        fn stepping_world() -> World {
            let mut world = World::new();
            world.init_resource::<Runs>();

            let mut schedule = Schedule::new(TestSchedule);
            schedule.add_systems((first_system, second_system).chain());
            world.add_schedule(schedule);

            let mut stepping = Stepping::new();
            stepping.add_schedule(TestSchedule);
            world.insert_resource(stepping);

            world
        }

        fn run_frame(world: &mut World) {
            let mut system_state: SystemState<Option<ResMut<Stepping>>> = SystemState::new(world);
            let stepping = system_state.get_mut(world).unwrap();
            Stepping::begin_frame(stepping);
            world.run_schedule(TestSchedule);
        }

        #[test]
        fn stepping_status_without_resource_errors() {
            let error = stepping_status(In(None), &World::new()).unwrap_err();
            assert_eq!(error.code, crate::error_codes::RESOURCE_ERROR);
        }

        #[test]
        fn stepping_enable_applies_next_frame() {
            let mut world = stepping_world();
            run_frame(&mut world);
            assert_eq!(take_runs(&mut world), ["first", "second"]);

            assert_eq!(stepping_enable(In(None), &mut world).unwrap(), Value::Null);
            assert!(!status(&world).enabled);

            run_frame(&mut world);
            let status = status(&world);
            assert!(status.enabled);
            assert_eq!(status.schedules, vec![format!("{TestSchedule:?}")]);
            assert!(take_runs(&mut world).is_empty());
        }

        #[test]
        fn stepping_disable_applies_next_frame() {
            let mut world = stepping_world();
            stepping_enable(In(None), &mut world).unwrap();
            run_frame(&mut world);
            assert!(take_runs(&mut world).is_empty());

            assert_eq!(stepping_disable(In(None), &mut world).unwrap(), Value::Null);
            assert!(status(&world).enabled);

            run_frame(&mut world);
            assert!(!status(&world).enabled);
            assert_eq!(take_runs(&mut world), ["first", "second"]);
        }

        #[test]
        fn stepping_step_frame_runs_one_system() {
            let mut world = stepping_world();
            stepping_enable(In(None), &mut world).unwrap();
            run_frame(&mut world);
            assert!(cursor_system(&world).unwrap().ends_with("first_system"));

            assert_eq!(
                stepping_step_frame(In(None), &mut world).unwrap(),
                Value::Null
            );
            run_frame(&mut world);
            assert_eq!(take_runs(&mut world), ["first"]);
            assert!(cursor_system(&world).unwrap().ends_with("second_system"));
        }

        #[test]
        fn stepping_continue_frame_runs_the_rest_of_the_frame() {
            let mut world = stepping_world();
            stepping_enable(In(None), &mut world).unwrap();
            run_frame(&mut world);
            stepping_step_frame(In(None), &mut world).unwrap();
            run_frame(&mut world);
            assert_eq!(take_runs(&mut world), ["first"]);

            assert_eq!(
                stepping_continue_frame(In(None), &mut world).unwrap(),
                Value::Null
            );
            run_frame(&mut world);
            assert_eq!(take_runs(&mut world), ["second"]);
        }

        #[test]
        fn stepping_enable_without_resource_errors() {
            let mut world = World::new();
            assert!(stepping_enable(In(None), &mut world).is_err());
        }
    }
}
