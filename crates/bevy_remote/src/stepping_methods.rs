//! `stepping.*` verbs for the Bevy Remote Protocol.

use bevy_ecs::schedule::{InternedScheduleLabel, NodeId, Schedules, Stepping};
use bevy_ecs::system::In;
use bevy_ecs::world::World;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BrpError, BrpResult};

/// The method path for a `stepping.status` request.
pub const BRP_STEPPING_STATUS: &str = "stepping.status";

/// The method path for a `stepping.enable` request.
pub const BRP_STEPPING_ENABLE: &str = "stepping.enable";

/// The method path for a `stepping.disable` request.
pub const BRP_STEPPING_DISABLE: &str = "stepping.disable";

/// The method path for a `stepping.step_frame` request.
pub const BRP_STEPPING_STEP_FRAME: &str = "stepping.step_frame";

/// The method path for a `stepping.continue_frame` request.
pub const BRP_STEPPING_CONTINUE_FRAME: &str = "stepping.continue_frame";

/// The position of the stepping cursor within the stepping frame.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpSteppingCursor {
    /// The label of the schedule the cursor is currently in.
    pub schedule: String,

    /// The name of the system the cursor is currently at.
    pub system: String,
}

/// The response to any of the `stepping.*` requests.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct BrpSteppingStatusResponse {
    /// Whether system stepping is currently enabled.
    pub enabled: bool,

    /// The labels of the schedules that have been added to [`Stepping`].
    pub schedules: Vec<String>,

    /// The current position of the stepping cursor, if there is one.
    pub cursor: Option<BrpSteppingCursor>,
}

/// Handles a `stepping.status` request coming from a client.
pub fn stepping_status(In(_params): In<Option<Value>>, world: &World) -> BrpResult {
    serde_json::to_value(build_stepping_status(world)).map_err(BrpError::internal)
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

/// Applies `update` to the [`Stepping`] resource and responds with the new status.
fn update_stepping(world: &mut World, update: fn(&mut Stepping) -> &mut Stepping) -> BrpResult {
    {
        let Some(mut stepping) = world.get_resource_mut::<Stepping>() else {
            return Err(BrpError::resource_error(
                "The `Stepping` resource is not present in the world",
            ));
        };
        update(&mut stepping);
    }

    serde_json::to_value(build_stepping_status(world)).map_err(BrpError::internal)
}

/// Collects the current state of the [`Stepping`] resource, if it is present.
fn build_stepping_status(world: &World) -> BrpSteppingStatusResponse {
    let Some(stepping) = world.get_resource::<Stepping>() else {
        return BrpSteppingStatusResponse::default();
    };

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
mod stepping {
    use super::*;
    use bevy_ecs::schedule::{Schedule, ScheduleLabel};
    use bevy_ecs::system::{ResMut, SystemState};

    #[derive(ScheduleLabel, Hash, Clone, PartialEq, Eq, Debug)]
    struct TestSchedule;

    fn status(world: &World) -> BrpSteppingStatusResponse {
        let value = stepping_status(In(None), world).unwrap();
        serde_json::from_value(value).unwrap()
    }

    fn stepping_world() -> World {
        let mut world = World::new();

        let mut schedule = Schedule::new(TestSchedule);
        schedule.add_systems(|| {});
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
    fn stepping_status_without_resource() {
        let status = status(&World::new());
        assert!(!status.enabled);
        assert!(status.schedules.is_empty());
        assert!(status.cursor.is_none());
    }

    #[test]
    fn stepping_enable_reports_schedule() {
        let mut world = stepping_world();
        stepping_enable(In(None), &mut world).unwrap();
        run_frame(&mut world);

        let status = status(&world);
        assert!(status.enabled);
        assert_eq!(status.schedules, vec![format!("{TestSchedule:?}")]);
    }

    #[test]
    fn stepping_disable_reports_disabled() {
        let mut world = stepping_world();
        stepping_enable(In(None), &mut world).unwrap();
        run_frame(&mut world);
        stepping_disable(In(None), &mut world).unwrap();
        run_frame(&mut world);

        assert!(!status(&world).enabled);
    }

    #[test]
    fn stepping_frame_controls_succeed() {
        let mut world = stepping_world();
        stepping_enable(In(None), &mut world).unwrap();
        run_frame(&mut world);

        assert!(stepping_step_frame(In(None), &mut world).is_ok());
        run_frame(&mut world);
        assert!(stepping_continue_frame(In(None), &mut world).is_ok());
        run_frame(&mut world);
    }

    #[test]
    fn stepping_enable_without_resource_errors() {
        let mut world = World::new();
        assert!(stepping_enable(In(None), &mut world).is_err());
    }
}
