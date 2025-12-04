//! Machine animation system.

use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_time::Time;
use bevy_transform::components::Transform;

use super::components::{Machine, MachineActivity, Mobility};
use super::MachineEvent;

/// System that updates machine positions and states based on their activities.
pub fn animation_system(
    time: Res<Time>,
    mut machines: Query<(
        Entity,
        &Machine,
        &Mobility,
        &mut MachineActivity,
        &mut Transform,
    )>,
    mut events: MessageWriter<MachineEvent>,
) {
    let dt = time.delta_secs();

    for (entity, machine, mobility, mut activity, mut transform) in machines.iter_mut() {
        match activity.as_mut() {
            MachineActivity::Idle => {
                // Nothing to do
            }
            MachineActivity::Traveling {
                target,
                progress,
                start,
            } => {
                let distance = target.distance(*start);
                if distance > 0.001 {
                    let travel_time = distance / mobility.max_speed;
                    *progress += dt / travel_time;

                    if *progress >= 1.0 {
                        *progress = 1.0;
                        transform.translation = *target;
                        events.write(MachineEvent::ReachedDestination { entity });

                        // Transition to idle
                        *activity = MachineActivity::Idle;
                    } else {
                        // Interpolate position
                        transform.translation = start.lerp(*target, *progress);

                        // Update rotation to face target
                        let direction = (*target - *start).normalize();
                        if direction.length_squared() > 0.001 {
                            let target_rotation = bevy_math::Quat::from_rotation_y(
                                (-direction.x).atan2(-direction.z),
                            );
                            transform.rotation = transform
                                .rotation
                                .slerp(target_rotation, (mobility.turn_rate * dt).min(1.0));
                        }
                    }
                } else {
                    *activity = MachineActivity::Idle;
                }
            }
            MachineActivity::Excavating {
                target,
                progress,
                volume,
            } => {
                // Excavation takes ~2 seconds base time, scaled by volume
                let excavate_time = 2.0 + *volume * 0.5;
                *progress += dt / excavate_time;

                if *progress >= 1.0 {
                    *progress = 1.0;
                    events.write(MachineEvent::CompletedExcavating {
                        entity,
                        volume: *volume,
                    });
                    *activity = MachineActivity::Idle;
                }
            }
            MachineActivity::Dumping {
                target,
                progress,
                volume,
            } => {
                // Dumping takes ~1.5 seconds base time
                let dump_time = 1.5 + *volume * 0.3;
                *progress += dt / dump_time;

                if *progress >= 1.0 {
                    *progress = 1.0;
                    events.write(MachineEvent::CompletedDumping {
                        entity,
                        volume: *volume,
                    });
                    *activity = MachineActivity::Idle;
                }
            }
            MachineActivity::Pushing {
                direction,
                progress,
            } => {
                // Pushing is continuous, progress represents one push cycle
                *progress += dt / 3.0; // 3 second push cycle

                if *progress >= 1.0 {
                    *progress = 0.0; // Reset for next cycle
                }
            }
        }
    }
}

/// Starts a machine moving to a target position.
pub fn start_move(
    activity: &mut MachineActivity,
    current_pos: Vec3,
    target: Vec3,
    events: &mut MessageWriter<MachineEvent>,
    entity: Entity,
) {
    events.write(MachineEvent::StartedMoving { entity, target });
    *activity = MachineActivity::Traveling {
        target,
        progress: 0.0,
        start: current_pos,
    };
}

/// Starts an excavation operation.
pub fn start_excavate(
    activity: &mut MachineActivity,
    target: Vec3,
    volume: f32,
    events: &mut MessageWriter<MachineEvent>,
    entity: Entity,
) {
    events.write(MachineEvent::StartedExcavating { entity });
    *activity = MachineActivity::Excavating {
        target,
        progress: 0.0,
        volume,
    };
}

/// Starts a dump operation.
pub fn start_dump(
    activity: &mut MachineActivity,
    target: Vec3,
    volume: f32,
    events: &mut MessageWriter<MachineEvent>,
    entity: Entity,
) {
    events.write(MachineEvent::StartedDumping { entity });
    *activity = MachineActivity::Dumping {
        target,
        progress: 0.0,
        volume,
    };
}
