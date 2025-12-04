//! Plan execution system.

use bevy_ecs::prelude::*;
use bevy_time::Time;

use super::playback::PlanPlayback;
use super::schema::{ExecutionPlan, PlannedAction};
use super::{PlanStepEvent, StepResult};
use crate::machines::{Machine, MachineActivity, MachineEvent};

/// System that executes plan steps based on the current playback time.
pub fn plan_executor_system(
    time: Res<Time>,
    plans: Res<bevy_asset::Assets<ExecutionPlan>>,
    mut playback: ResMut<PlanPlayback>,
    mut machines: Query<(
        Entity,
        &Machine,
        &mut MachineActivity,
        &bevy_transform::components::Transform,
    )>,
    mut step_events: MessageWriter<PlanStepEvent>,
    mut machine_events: MessageWriter<MachineEvent>,
) {
    // Advance playback time
    let _reached_end = playback.advance(time.delta_secs());

    // Get the current plan
    let Some(plan_handle) = playback.plan_handle() else {
        return;
    };
    let Some(plan) = plans.get(plan_handle) else {
        return;
    };

    // Check for steps that should be triggered
    let current_time = playback.current_time();

    for (index, step) in plan.steps.iter().enumerate() {
        // Skip already executed steps
        if playback.is_step_executed(index) {
            continue;
        }

        // Check if this step should trigger
        if step.timestamp <= current_time {
            // Find the machine for this step
            let machine_entity = machines
                .iter()
                .find(|(_, m, _, _)| m.id == step.machine_id)
                .map(|(e, _, _, _)| e);

            let result = if let Some(entity) = machine_entity {
                // Execute the action
                if let Ok((_, machine, mut activity, transform)) = machines.get_mut(entity) {
                    execute_action(
                        entity,
                        &step.action,
                        &mut activity,
                        transform.translation,
                        &mut machine_events,
                    );
                    StepResult::Success
                } else {
                    StepResult::Failed("Machine not found".to_string())
                }
            } else {
                StepResult::Failed(format!("Machine '{}' not found", step.machine_id))
            };

            // Emit step event
            step_events.write(PlanStepEvent {
                step_index: index,
                step: step.clone(),
                result,
            });

            // Mark step as executed
            playback.mark_step_executed(index);
        }
    }
}

/// Executes a planned action on a machine.
fn execute_action(
    entity: Entity,
    action: &PlannedAction,
    activity: &mut MachineActivity,
    current_pos: bevy_math::Vec3,
    events: &mut MessageWriter<MachineEvent>,
) {
    match action {
        PlannedAction::MoveTo { target } => {
            events.write(MachineEvent::StartedMoving {
                entity,
                target: *target,
            });
            *activity = MachineActivity::Traveling {
                target: *target,
                progress: 0.0,
                start: current_pos,
            };
        }
        PlannedAction::Excavate { target, volume, .. } => {
            events.write(MachineEvent::StartedExcavating { entity });
            *activity = MachineActivity::Excavating {
                target: *target,
                progress: 0.0,
                volume: *volume,
            };
        }
        PlannedAction::Dump { target, volume } => {
            events.write(MachineEvent::StartedDumping { entity });
            *activity = MachineActivity::Dumping {
                target: *target,
                progress: 0.0,
                volume: *volume,
            };
        }
        PlannedAction::Push { direction, .. } => {
            *activity = MachineActivity::Pushing {
                direction: *direction,
                progress: 0.0,
            };
        }
        PlannedAction::Idle { .. } => {
            *activity = MachineActivity::Idle;
        }
        PlannedAction::WaitFor { .. } => {
            // WaitFor is handled by the executor checking dependencies
            // For now, just idle
            *activity = MachineActivity::Idle;
        }
    }
}
