//! AI gameplay agent for automated testing.
//!
//! This module provides an autonomous AI agent that can play the game,
//! making it possible to:
//! - Test gameplay mechanics automatically
//! - Verify job completion logic
//! - Profile performance under realistic gameplay conditions
//! - Create demo recordings

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::{IVec3, Quat, Vec3};
use bevy_reflect::Reflect;
use bevy_time::Time;
use bevy_transform::components::Transform;

use crate::jobs::{CurrentJob, JobType};
use crate::machines::{BladeState, Machine, MachineActivity, MachineType, Mobility};
use crate::terrain::{get_terrain_height, Chunk, VoxelTerrain};

/// AI agent component for autonomous machine control.
#[derive(Component, Clone, Debug, Reflect)]
pub struct AIAgent {
    /// Current behavior mode.
    pub behavior: AgentBehavior,
    /// Current goal being pursued.
    pub current_goal: Option<AgentGoal>,
    /// Time spent on current goal.
    pub goal_time: f32,
    /// Maximum time to spend on a goal before re-evaluating.
    pub goal_timeout: f32,
    /// Agent state machine.
    pub state: AgentState,
}

impl Default for AIAgent {
    fn default() -> Self {
        Self {
            behavior: AgentBehavior::CompleteJob,
            current_goal: None,
            goal_time: 0.0,
            goal_timeout: 30.0, // 30 seconds max per goal
            state: AgentState::Idle,
        }
    }
}

impl AIAgent {
    /// Creates an agent that will complete jobs.
    pub fn job_completer() -> Self {
        Self {
            behavior: AgentBehavior::CompleteJob,
            ..Default::default()
        }
    }

    /// Creates an agent that follows a scripted path.
    pub fn scripted(waypoints: Vec<Vec3>) -> Self {
        Self {
            behavior: AgentBehavior::Scripted(waypoints),
            ..Default::default()
        }
    }

    /// Creates a random exploration agent.
    pub fn explorer() -> Self {
        Self {
            behavior: AgentBehavior::Random,
            ..Default::default()
        }
    }
}

/// Agent behavior mode.
#[derive(Clone, Debug, Default, Reflect)]
pub enum AgentBehavior {
    /// Random exploration and digging.
    Random,
    /// Work to complete the current job.
    #[default]
    CompleteJob,
    /// Maximize Zyns earned.
    Optimize,
    /// Follow specific waypoints.
    Scripted(Vec<Vec3>),
}

/// Current goal the agent is pursuing.
#[derive(Clone, Debug, Reflect)]
pub enum AgentGoal {
    /// Move to a specific position.
    MoveTo(Vec3),
    /// Level an area to target height.
    LevelArea {
        /// Minimum corner.
        min: IVec3,
        /// Maximum corner.
        max: IVec3,
        /// Target height.
        target_height: i32,
    },
    /// Excavate material at a position.
    Excavate(Vec3),
    /// Dump material at a position.
    Dump(Vec3),
    /// Wait for a duration.
    Wait(f32),
}

/// Agent state machine states.
#[derive(Clone, Debug, Default, Reflect, PartialEq)]
pub enum AgentState {
    /// Waiting for a goal.
    #[default]
    Idle,
    /// Moving to a location.
    Moving,
    /// Excavating terrain.
    Excavating,
    /// Dumping material.
    Dumping,
    /// Waiting.
    Waiting,
    /// Goal completed.
    GoalComplete,
}

/// Plugin for AI agent systems.
pub struct AIAgentPlugin;

impl Plugin for AIAgentPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AIAgent>()
            .add_systems(
                Update,
                (
                    ai_agent_goal_selection,
                    ai_agent_decision,
                    ai_agent_execution,
                )
                    .chain(),
            );
    }
}

/// Select goals based on current behavior.
fn ai_agent_goal_selection(
    time: Res<Time>,
    job: Option<Res<CurrentJob>>,
    terrain: Res<VoxelTerrain>,
    chunks: Query<&Chunk>,
    mut agents: Query<(&mut AIAgent, &Transform)>,
) {
    let dt = time.delta_secs();

    for (mut agent, transform) in agents.iter_mut() {
        // Update goal timer
        agent.goal_time += dt;

        // Check if we need a new goal
        let need_new_goal = agent.current_goal.is_none()
            || agent.state == AgentState::GoalComplete
            || agent.goal_time > agent.goal_timeout;

        if !need_new_goal {
            continue;
        }

        // Reset goal timer
        agent.goal_time = 0.0;
        agent.state = AgentState::Idle;

        // Select new goal based on behavior
        let new_goal = match &agent.behavior {
            AgentBehavior::CompleteJob => {
                select_job_goal(&job, &terrain, &chunks, transform.translation)
            }
            AgentBehavior::Random => {
                select_random_goal(transform.translation)
            }
            AgentBehavior::Scripted(waypoints) => {
                select_scripted_goal(waypoints, transform.translation)
            }
            AgentBehavior::Optimize => {
                // For now, same as CompleteJob
                select_job_goal(&job, &terrain, &chunks, transform.translation)
            }
        };

        agent.current_goal = new_goal;
    }
}

/// Select a goal to work toward completing the current job.
fn select_job_goal(
    job: &Option<Res<CurrentJob>>,
    terrain: &VoxelTerrain,
    chunks: &Query<&Chunk>,
    current_pos: Vec3,
) -> Option<AgentGoal> {
    let job = job.as_ref()?;
    let active_job = job.active.as_ref()?;

    match &active_job.job_type {
        JobType::LevelArea {
            min_x,
            max_x,
            min_z,
            max_z,
            target_height,
        } => {
            // Find a spot that needs work
            let work_pos = find_work_position(
                terrain,
                chunks,
                *min_x,
                *max_x,
                *min_z,
                *max_z,
                *target_height,
                current_pos,
            );

            work_pos.map(|pos| AgentGoal::MoveTo(pos))
        }
    }
}

/// Find a position within the job area that needs work.
fn find_work_position(
    terrain: &VoxelTerrain,
    chunks: &Query<&Chunk>,
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
    target_height: i32,
    current_pos: Vec3,
) -> Option<Vec3> {
    let voxel_size = terrain.voxel_size();
    let target_world_height = target_height as f32 * voxel_size;
    let mut best_pos: Option<Vec3> = None;
    let mut best_distance = f32::MAX;

    // Scan the area for spots that need work
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            let world_x = x as f32 * voxel_size;
            let world_z = z as f32 * voxel_size;

            if let Some(height) = get_terrain_height(terrain, chunks, world_x, world_z) {
                // Check if this spot needs excavation (too high)
                if height > target_world_height {
                    let world_pos = Vec3::new(world_x, height, world_z);

                    let distance = current_pos.distance(world_pos);
                    if distance < best_distance {
                        best_distance = distance;
                        best_pos = Some(world_pos);
                    }
                }
            }
        }
    }

    best_pos
}

/// Select a random exploration goal.
fn select_random_goal(current_pos: Vec3) -> Option<AgentGoal> {
    // Generate a random nearby position
    let angle = (current_pos.x * 1.618 + current_pos.z * 2.718).sin() * core::f32::consts::TAU;
    let distance = 5.0 + (current_pos.x * 0.7).sin().abs() * 10.0;

    let target = Vec3::new(
        current_pos.x + angle.cos() * distance,
        current_pos.y,
        current_pos.z + angle.sin() * distance,
    );

    Some(AgentGoal::MoveTo(target))
}

/// Select the next waypoint from a scripted path.
fn select_scripted_goal(waypoints: &[Vec3], current_pos: Vec3) -> Option<AgentGoal> {
    if waypoints.is_empty() {
        return None;
    }

    // Find nearest waypoint
    let mut nearest_idx = 0;
    let mut nearest_dist = f32::MAX;

    for (i, wp) in waypoints.iter().enumerate() {
        let dist = current_pos.distance(*wp);
        if dist < nearest_dist {
            nearest_dist = dist;
            nearest_idx = i;
        }
    }

    // Select next waypoint after nearest
    let next_idx = (nearest_idx + 1) % waypoints.len();
    Some(AgentGoal::MoveTo(waypoints[next_idx]))
}

/// Make decisions based on current goal.
fn ai_agent_decision(
    mut agents: Query<(&mut AIAgent, &Transform, &BladeState)>,
) {
    for (mut agent, transform, blade) in agents.iter_mut() {
        let Some(goal) = &agent.current_goal else {
            continue;
        };

        match goal {
            AgentGoal::MoveTo(target) => {
                let distance = transform.translation.distance(*target);

                if distance < 2.0 {
                    // Reached target, check if we should excavate
                    if blade.load < blade.capacity * 0.8 {
                        agent.state = AgentState::Excavating;
                    } else {
                        agent.state = AgentState::GoalComplete;
                    }
                } else {
                    agent.state = AgentState::Moving;
                }
            }
            AgentGoal::LevelArea { .. } => {
                agent.state = AgentState::Excavating;
            }
            AgentGoal::Excavate(_) => {
                if blade.is_full() {
                    agent.state = AgentState::GoalComplete;
                } else {
                    agent.state = AgentState::Excavating;
                }
            }
            AgentGoal::Dump(_) => {
                if blade.is_empty() {
                    agent.state = AgentState::GoalComplete;
                } else {
                    agent.state = AgentState::Dumping;
                }
            }
            AgentGoal::Wait(duration) => {
                if agent.goal_time >= *duration {
                    agent.state = AgentState::GoalComplete;
                } else {
                    agent.state = AgentState::Waiting;
                }
            }
        }
    }
}

/// Execute agent actions based on current state.
fn ai_agent_execution(
    time: Res<Time>,
    mut agents: Query<(
        &AIAgent,
        &mut Transform,
        &Mobility,
        &mut BladeState,
        &mut MachineActivity,
        &Machine,
    )>,
) {
    let dt = time.delta_secs();

    for (agent, mut transform, mobility, mut blade, mut activity, machine) in agents.iter_mut() {
        if machine.machine_type != MachineType::Dozer {
            continue;
        }

        let Some(goal) = &agent.current_goal else {
            *activity = MachineActivity::Idle;
            continue;
        };

        match agent.state {
            AgentState::Moving => {
                if let AgentGoal::MoveTo(target) = goal {
                    // Calculate direction to target
                    let direction = (*target - transform.translation).normalize();
                    let target_yaw = (-direction.x).atan2(-direction.z);

                    // Rotate toward target
                    let current_yaw = transform.rotation.to_euler(bevy_math::EulerRot::YXZ).0;
                    let yaw_diff = angle_diff(current_yaw, target_yaw);

                    if yaw_diff.abs() > 0.1 {
                        // Need to turn
                        let turn_amount = yaw_diff.signum() * mobility.turn_rate * dt;
                        transform.rotation =
                            transform.rotation * Quat::from_rotation_y(turn_amount);
                    }

                    // Move forward if roughly facing target
                    if yaw_diff.abs() < 0.5 {
                        let forward = transform.rotation * Vec3::NEG_Z;
                        transform.translation += forward * mobility.max_speed * dt;

                        *activity = MachineActivity::Traveling {
                            target: *target,
                            progress: 0.0,
                            start: transform.translation,
                        };
                    }

                    // Raise blade while traveling
                    blade.height = (blade.height + dt * 2.0).min(0.5);
                }
            }
            AgentState::Excavating => {
                // Lower blade to dig
                blade.height = (blade.height - dt * 2.0).max(-0.8);

                // Move forward slowly
                let forward = transform.rotation * Vec3::NEG_Z;
                transform.translation += forward * mobility.max_speed * 0.5 * dt;

                *activity = MachineActivity::Traveling {
                    target: transform.translation + forward * 5.0,
                    progress: 0.0,
                    start: transform.translation,
                };
            }
            AgentState::Dumping => {
                // Raise blade to dump
                blade.height = (blade.height + dt * 3.0).min(1.0);

                // Stop moving
                *activity = MachineActivity::Idle;
            }
            AgentState::Waiting | AgentState::Idle | AgentState::GoalComplete => {
                *activity = MachineActivity::Idle;
            }
        }
    }
}

/// Calculate the shortest angle difference between two angles.
fn angle_diff(from: f32, to: f32) -> f32 {
    let diff = (to - from) % core::f32::consts::TAU;
    if diff > core::f32::consts::PI {
        diff - core::f32::consts::TAU
    } else if diff < -core::f32::consts::PI {
        diff + core::f32::consts::TAU
    } else {
        diff
    }
}
